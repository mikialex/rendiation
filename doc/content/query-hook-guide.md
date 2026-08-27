# Query-Hook 模式理解（utility/query-hook）

本文档是 [hooks-guide.md](hooks-guide.md) 的姊妹篇，梳理 [utility/query-hook](../../utility/query-hook) 如何在 [utility/hook](../../utility/hook)（见 [hooks-guide.md](hooks-guide.md)）之上扩展出增量计算管线：两阶段执行模型、共享计算、唤醒传播与调度基础设施。阅读前建议先了解 [utility/query](../../utility/query) 的增量查询模型（DualQuery / DataChanges / ValueChange）。

## 模式概览

hook 运行时只提供"有状态、每轮执行"的骨架，query-hook 把 [utility/query](../../utility/query) 的增量查询接到这个骨架上：

- 每轮执行拆成 **spawn 阶段**（声明订阅、同步计算、收集异步任务）与 **resolve 阶段**（取回异步结果、消费变化），由上层驱动循环推进。
- 派生数据通过 `UseResult<T>` 四态在阶段间流动，任务以 token 注册、按 id 取回。
- 同一份派生计算（如全局世界矩阵）通过 `SharedResultProvider` 共享，只有一份 upstream，多个消费者复用结果并各自消费变化。
- 数据变化通过 waker 传播：数据库写、异步任务完成都会唤醒对应分支，下一轮只有被唤醒的分支重算。

## 核心机制

### 两阶段执行模型

[utility/query-hook/src/lib.rs](../../utility/query-hook/src/lib.rs) 定义 `QueryHookCxLike`（扩展 HooksCxLike + InspectableCx），核心是 `stage()` 返回的 `QueryHookStage`：

| 阶段 | 含义 |
| --- | --- |
| `SpawnTask` | 可以声明订阅、同步计算（SpawnStageReady）、安装异步任务（SpawnStageFuture） |
| `ResolveTask` | 按 token 取回任务结果（ResolveStageReady） |
| `Other` | 其余阶段（如 UI、场景写回） |

上层驱动循环（参照 [ViewerCx::stage_of_update](../../application/viewer/src/viewer/mod.rs)）：

```text
每帧循环（cycle_count 次，处理层级传递等收敛问题）：
  SpawnTask 阶段：cx.execute(internal)，收集异步任务到 AsyncTaskPool
  block_on(pool.all_async_task_done())：等待任务完成，结果进 TaskPoolResultCx
  EventHandling(resolve) 阶段：cx.execute(internal)，按 token 取结果
  SceneContentUpdate 阶段：cx.execute(internal)，把派生结果写回场景
```

每轮执行前需 `setup_new_frame_allocator` 重建帧分配器、`reset_visiting` 清空共享计算的跨轮状态。

### UseResult：跨阶段的结果传递

[utility/query-hook/src/use_result.rs](../../utility/query-hook/src/use_result.rs) 定义 `UseResult<T>` 四态：

| 状态 | 含义 |
| --- | --- |
| `SpawnStageFuture` | 需要异步计算，spawn 阶段安装到 AsyncTaskPool |
| `SpawnStageReady` | spawn 阶段同步可得 |
| `ResolveStageReady` | resolve 阶段可得（任务结果或共享缓存） |
| `NotInStage` | 当前阶段无结果 |

组合子：

- `map`：同时映射所有阶段；`filter_map_changes` / `map_changes` 是针对 DataChanges 的便捷映射。
- `join`：合并两个结果，要求同为 resolve 或同为 spawn-ready，混合会 panic（"join source corrupted"）。
- `fork`：把 future 转为 shared 供多处等待。
- `map_spawn_stage_in_thread(cx, should_spawn, f)`：核心转换——若结果有变化（should_spawn 谓词判定），把后续计算 `spawner.spawn_task(...)` 丢到 worker 线程池返回 SpawnStageFuture；无变化则原地同步 map。它只在 spawn 阶段变换，resolve 阶段透传为 NotInStage。
- `use_assure_result(cx)`：消费者收口——spawn 阶段把 future 安装为 token 任务（`install_task`），resolve 阶段按 token 取回 ResolveStageReady。
- `use_retain_view_to_resolve_stage`：spawn 阶段把 DualQuery 的 view 暂存，resolve 阶段取回。
- `use_validation` / `fanout` / `dual_query_*` 系列：DualQuery 上的变换与校验（materialize、zip、intersect、union、cross_join、reverse 等），配合 query 系统的算子使用。

### 共享计算

`SharedResultProvider<Cx>` 定义一份可共享的派生计算：

```rust
impl<Cx: DBHookCxLike> SharedResultProvider<Cx> for GlobalNodeConnectivity {
  type Result = RevRefContainerRead<RawEntityHandle, RawEntityHandle>;
  share_provider_hash_type_id! {} // 以 Self 的 TypeId 作为共享 key

  fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> { ... }
}
```

- `compute_share_key()`（ShareKey::TypeId 或 Hash）决定共享身份，同一 key 全局只有一份 upstream（SharedHookObject：独立 FunctionMemory + consumer 集合 + BroadcastWaker）。
- 第一个消费者触发 `use_shared_compute_internal_dyn` 的 `enter_shared_ctx`：swap 到共享记忆执行 use_logic（订阅、use_plain_state 都落在共享记忆里），结果按情形处理：SpawnStageFuture 安装为任务存入 task_id_mapping；SpawnStageReady 存 adhoc id 进 immediate_results；ResolveStageReady 存进 token_based_result；NotInStage 标记 u32::MAX 表示"已查过 upstream"。之后其他消费者直接复用结果或共享任务。
- `use_shared_dual_query` 额外接入 `SharedQueryChangeReconciler`：delta 只能被一个消费者消费，第一个消费者拿到后分发给其余消费者的缓冲（broadcast），各自 `reconcile` 时合并取出（`finalize_buffered_changes`）。`use_shared_dual_query_view` 只共享 view（skip_change，不传 delta，但仍正确唤醒）。
- 消费者持有 `SharedConsumerToken`，其 Drop 把 (key, consumer_id) 推入 drop_queue，在 `flush_drop_queue` 中真正移除；最后一个消费者离开时销毁共享状态。刻意不用 cx 的 cleanup 机制，因为共享记忆可能被多个不同 Cx 访问，无法绑定单一 drop context。
- `use_shared_hash_map` / `maintain_shared_map`：跨帧共享的 HashMap，用于把变化增量物化为 map 视图。

### 唤醒传播

所有异步/数据库变化都通过 waker 传播：

- `ChangeNotifier`（[utility/query-hook/src/wake_util.rs](../../utility/query-hook/src/wake_util.rs)）：AtomicBool + AtomicWaker。`run_with_waked_info` 检查自上次以来是否被唤醒并返回，`skip_if_not_waked` 据此跳过未唤醒的计算分支（未唤醒返回 None 且保留 scope slot；若 spawn 阶段执行过则自动 wake_by_ref 保证 resolve 阶段也执行）。它是基于 `skip_if_not` 的**优化**：把可能跳过的整段逻辑包进 scope，数据未变化时省去执行成本；未唤醒时内部由 scope 保持状态形状（注意首帧 changed 初值为 true，保证全量执行）。
- `BroadcastWaker`：一个共享计算唤醒全部消费者（每轮只广播一次）。
- `use_begin_change_set_collect`：把当前 waker 换成 notifier 的 waker，收集一个 hook scope 内是否发生过任何变化，结束时恢复 waker 并返回标志。用于"这段逻辑本轮是否有资源变化"的判断，避免无变化时仍重算 GPU 资源。

### 调度基础设施

- [utility/query-hook/src/frame_allocator.rs](../../utility/query-hook/src/frame_allocator.rs)：thread_local 的 bumpalo 帧分配器（FrameAlloc），所有 spawn 出的 future 与结果在帧内分配、帧结束整体释放。`box_in_frame` / `pin_box_in_frame` 是其入口，`get_global_living_bump` 统计存活的帧 bump。
- [utility/query-hook/src/task_pool.rs](../../utility/query-hook/src/task_pool.rs)：`TaskSpawner` 封装 rayon 线程池（wasm 下退化为同步执行）；`AsyncTaskPool` 以 u32 token 注册共享任务（`install_task` / `try_share_task_by_id`）；`TaskPoolResultCx` 按 token 存结果，`all_async_task_done` 收集全部任务结果。

### 数据源扩展（DBHookCxLike）

[utility/database/src/hook/mod.rs](../../utility/database/src/hook/mod.rs) 把数据库变化监听接到 hook 的 waker 上，是增量计算的数据源头：

- `use_changes` / `use_dual_query` / `use_query_change` / `use_entity_set_delta_raw`：订阅组件/实体变化。spawn 阶段 poll 变化（有变化返回 SpawnStageReady 或安装计算任务），resolve 阶段若 has_change 则 wake 自己保证下一轮执行。
- `use_db_rev_ref` / `use_db_rev_ref_tri_view`：外键的反向引用（many-to-one），维护 DenseIndexMapping。
- 数据库写入通过组件 data_watchers 事件源唤醒注册的 waker。

## 使用规则

### UseResult 消费规则

1. **在正确的阶段消费结果**：spawn 阶段处理 SpawnStageReady / SpawnStageFuture（安装或继续组合）；resolve 阶段取 ResolveStageReady（`expect_resolve_stage` / `if_ready`）。NotInStage 表示当前阶段无结果，组合链自动透传，不要在未检查的状态上 unwrap。
2. **消费者收口统一用 `use_assure_result`**：spawn 阶段安装任务、resolve 阶段按 token 取回，得到 ResolveStageReady 后在渲染/事件阶段使用；需要把 view 保到 resolve 阶段用 `use_retain_view_to_resolve_stage`。
3. **`map` 同时映射 spawn 与 resolve 阶段，`map_spawn_stage_in_thread` 只在 spawn 阶段变换**。若结果内含一次性消费的 change，不要在 spawn 阶段之外重复消费（change 已被 map 消费，重复使用会造成逻辑错误）。
4. **`join` 只允许同为 resolve 或同为 spawn-ready 的结果**，混合阶段 panic。
5. **should_spawn 谓词决定是否走线程**：只有变化（has_change / has_delta_hint / has_item_hint）时才值得切线程计算；无变化直接同步 map。

### 共享计算规则

1. **provider 的共享 key 必须全局稳定唯一**：默认用 `share_provider_hash_type_id!`（Self 的 TypeId）；自定义 key（如组件/实体维度）需保证等价性（如 DBDualQueryProvider 用组件 TypeId）。同 key 不同 provider 会被视为同一份计算，会串数据。
2. **`use_logic` 内部的 hook 调用顺序必须对所有消费者一致**：共享记忆按顺序寻址，不同消费者若在 use_logic 中走不同分支/顺序，状态会错位。
3. **delta/change 不可共享**：每个消费者必须各自消费 delta，共享 DualQuery 依赖 reconciler 广播。默认 `use_shared_dual_query` 返回完整 delta；只读 view 用 `use_shared_dual_query_view`（skip_change，仍正确唤醒）。
4. **不要在共享计算里放需要 cx 清理的资源**（共享记忆只支持 plain state），其生命周期由 SharedConsumerToken 的 Drop + drop_queue 管理。
5. **每轮驱动循环开头 `reset_visiting`**（清空 task_id_mapping、reconciler 复位），否则共享计算的任务 id 与广播状态跨轮残留；`flush_drop_queue` 在合适时机（如 UI 阶段）调用处理消费者销毁。

### 唤醒与数据源规则

1. **`skip_if_not_waked` 是优化而非分支处理手段**：未唤醒时整个闭包被跳过（内部 scope 保持形状），只有被数据变化唤醒的分支才执行；spawn 阶段执行过时它保证 resolve 阶段也执行。分支隔离本身仍由 scope 承担。
2. **`use_begin_change_set_collect` 的 scope ender 必须在同一 hook scope 内调用**（源码注释约定，暂缺运行时校验）。
3. **数据库订阅用 DBHookCxLike 的接口**（use_changes / use_dual_query / use_db_rev_ref），不要绕过 hook 直接轮询数据库，否则失去增量调度。

### Cx 实现与驱动规则

1. **实现 `QueryHookCxLike` 必须把自定义 stage 映射到 `QueryHookStage`**，并实现 `shared_hook_ctx`（共享 SharedHooksCtx）、`waker`、`use_shared_consumer`（创建 SharedConsumerToken）。参照 [QueryGPUHookCx](../../platform/graphics/webgpu-hook-utils/src/hook.rs) 与 [ViewerAPICx](../../application/viewer-content-api/src/cx.rs)。
2. **驱动循环是框架运行的前提**：spawn 阶段收集任务后必须 await 全部任务再进入 resolve 阶段（`all_async_task_done`），且每轮执行前重建帧分配器（`setup_new_frame_allocator`）。

## 典型使用模板

定义一份共享派生（provider 侧），参照 [scene/core/src/node.rs](../../scene/core/src/node.rs)：

```rust
pub struct GlobalNodeDerive<F, C>(pub F, PhantomData<C>);

impl<C, Cx, F> SharedResultProvider<Cx> for GlobalNodeDerive<F, C>
where C: ComponentSemantic, Cx: DBHookCxLike,
      F: Fn(&C::Data, Option<&C::Data>) -> C::Data + Send + Sync + 'static + Copy,
{
  type Result = DeriveDataDualQuery<C::Data>;
  share_provider_hash_type_id! {}

  fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
    let connectivity_change = use_connectivity_change(cx);
    let payload_change = cx.use_query_change::<C>();
    let derived = cx.use_shared_hash_map::<RawEntityHandle, C::Data>("derived node data");

    connectivity_change
      .join(payload_change)
      .map_spawn_stage_in_thread(
        cx,
        |(a, b)| a.has_item_hint() || b.has_item_hint(),
        move |(a, b)| {
          // 增量更新 derived map，产出 DualQuery { view, delta }
        },
      )
  }
}
```

消费（消费者侧），参照 [application/viewer-content/src/pick.rs](../../application/viewer-content/src/pick.rs)：

```rust
let node_world = use_global_node_world_mat_view(cx).use_assure_result(cx);
// ... spawn 阶段安装任务，resolve/渲染阶段：
cx.when_render(|| {
  let world = node_world.expect_resolve_stage();
  // 使用 view
})
```

## 与 query 系统的关系

query-hook 是 [utility/query](../../utility/query) 增量查询系统的"运行时宿主"：query 层定义纯数据结构的变换（DualQuery/DataChanges/ValueChange），query-hook 层负责调度时机（两阶段）、内存生命周期（帧分配、状态记忆）、共享与广播（provider/reconciler）以及变化传播（waker）。编写新的派生逻辑时，先想清楚它属于哪一层：纯变换放 query，需要订阅/调度/共享的放 hook。
