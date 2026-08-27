# Rendiation GPU 任务图执行运行时指南（shader/task-graph）

本文梳理 [shader/task-graph](../../shader/task-graph/src/lib.rs) 的 GPU 任务图执行运行时：一套把"任务"组织成图、在 GPU 上以 compute shader 形式持续轮询执行的动态任务调度器。任务可以生成子任务、等待子任务、被唤醒、回收复用，整个生命周期都发生在 GPU 侧，宿主只负责按轮次驱动派发。当前唯一的下游消费者是 [shader/ray-tracing](../../shader/ray-tracing/src/lib.rs) 的 wavefront 光线追踪后端（shader/ray-tracing/src/backend/wavefront_compute/），它用这套运行时把"发射射线 → 遍历 BVH → 命中/未命中着色 → 递归追踪"组织成一张运行时图。

## 前置阅读

本 crate 用 shader EDSL 写 compute shader，并依赖 parallel-compute 的并行原语（运行时自身用流压缩做任务压缩）：

| 文档 | 内容 |
| --- | --- |
| [skill-translation/shader-edsl-compute-zh.md](skill-translation/shader-edsl-compute-zh.md) | 计算管线构建（`get_or_cache_create_compute_pipeline_by`）、工作组共享内存、`storage_barrier`、GPU 单元测试（`#[pollster::test]` + 回读） |
| [skill-translation/shader-edsl-core-zh.md](skill-translation/shader-edsl-core-zh.md) | `Node<T>` / `ShaderPtrOf`、`if_by` / `loop_by` / `switch_by`、`make_local_var`、原子操作、`storage_barrier` 等语言基础 |
| [skill-translation/shader-edsl-binding-and-typed-container-zh.md](skill-translation/shader-edsl-binding-and-typed-container-zh.md) | `StorageBufferDataView` / `UniformBufferDataView`、`bind_by` 与 pass 侧 `bind` |
| [parallel-compute-primitives-guide.md](parallel-compute-primitives-guide.md) | `DeviceParallelComputeCtx`（record_pass / 回读 / 提交）、`use_stream_compaction`（任务压缩直接用）、`compute_dispatch_size` |

## 模式概览

传统 GPU 光线追踪（硬件 RT 管线）的着色器切换与递归由硬件/驱动处理；wavefront 风格的后端没有这些设施，需要自己在 compute shader 里调度"哪条光线现在该跑哪个着色器"。任务图就是为此设计的通用机制：

- **任务分组**：图由若干任务组（task group，节点）组成，每个任务组有独立的类型化任务池（payload 类型、状态结构体、并发上限）。组与组之间通过"子任务 + 父任务引用"形成边。
- **任务**：池中的一个槽位，有自己的 payload、状态结构体与父任务引用。任务由宿主侧 spawner pass 或 GPU 侧父任务生成。
- **轮询执行**：宿主按轮次驱动。每一轮对每个任务组派发一个 compute pass，为活跃列表里的每个任务调用一次该组的 `ShaderFutureInvocation::device_poll`（对应任务的"一次性逻辑"）。任务可以解析（完成）、休眠、生成子任务。
- **图推进**：子任务完成后在 GPU 侧唤醒父任务（把父任务重新加入活跃列表），下一轮父任务被再次轮询，读到子任务结果后继续。一轮内完成的解析会在轮末压缩掉，任务索引回收进空闲池复用。
- **类型擦除**：任务的定义（future）以 `Box<dyn ShaderFuture>` 形式注册，输出统一擦除为 `Box<dyn Any>`；状态字段的类型用 `DynamicTypeBuilder` 在构建期逐字段动态构建，host 布局与 GPU 布局来自同一份元数据。

## 核心概念

| 概念 | 定义 | 说明 |
| --- | --- | --- |
| `DeviceTaskGraphBuildSource` | [runtime/mod.rs:53](../../shader/task-graph/src/runtime/mod.rs#L53) | 构建期描述：`define_task` / `define_task_dyn` 注册任务组，`capacity` 是每个任务组的容量乘数，`build` 产出执行器 |
| `DeviceTaskGraphExecutor` | [runtime/mod.rs:172](../../shader/task-graph/src/runtime/mod.rs#L172) | 执行期驱动：`execute` 定点轮询、`dispatch_allocate_init_task` 宿主侧播种任务、`read_back_execution_states` / `debug_execution` 观测 |
| `TaskGroupExecutor` | [runtime/task_group.rs:10](../../shader/task-graph/src/runtime/task_group.rs#L10) | 单个任务组：轮询管线、资源、执行前后钩子 |
| `TaskGroupExecutorResource` | [runtime/task_group.rs:304](../../shader/task-graph/src/runtime/task_group.rs#L304) | 任务组的 GPU 资源：活跃索引 bump 分配器、空闲池、待回收队列、任务池 |
| `TaskPool` | [runtime/task_pool.rs:4](../../shader/task-graph/src/runtime/task_pool.rs#L4) | 任务槽位数组，每个槽位是 `{is_finished, payload, state, parent_task_type_id, parent_task_index}` 结构体 |
| `TaskParentRef` | [runtime/task_pool.rs:109](../../shader/task-graph/src/runtime/task_pool.rs#L109) | 父任务引用：`parent_task_index` + `parent_task_type_id`（`none_parent()` 为 `u32::MAX`） |
| `ShaderFuture` | [future/mod.rs:68](../../shader/task-graph/src/future/mod.rs#L68) | 任务定义（build 期）：`build_poll` 构建调用实例、`bind_input` 绑定输入、`required_poll_count` 预估轮询次数 |
| `ShaderFutureInvocation` | [future/mod.rs:36](../../shader/task-graph/src/future/mod.rs#L36) | 任务逻辑（执行期）：`device_poll` 一次轮询，返回 `ShaderPoll { resolved, payload }` |
| `TaskFuture<T>` | [future/task.rs:2](../../shader/task-graph/src/future/task.rs#L2) | 等待另一任务组子任务的组合子：轮询子任务、读取其 payload |
| `DeviceBumpAllocationInstance<T>` | [bump_allocator.rs:4](../../shader/task-graph/src/bump_allocator.rs#L4) | GPU 侧 bump 分配器（两阶段：线程内 atomic bump + 派发间 commit） |
| `DynamicTypeBuilder` | [dyn_ty_builder.rs:3](../../shader/task-graph/src/dyn_ty_builder.rs#L3) | 动态状态结构体构建器：逐字段声明类型与默认值，指针延迟解析 |

## 数据流总览

```text
宿主（每帧 / 每次 trace_ray）
  └─ DeviceTaskGraphBuildSource::define_task（注册任务组：future + payload 类型 + max_in_flight）
       └─ build(cx)：为每个任务组分配 GPU 资源、构建轮询管线（compute shader）
            └─ DeviceTaskGraphExecutor
                 ├─ dispatch_allocate_init_task：GPU pass 播种初始任务（spawner 从全局 id 生成 payload）
                 └─ execute(round_count)：定点循环，每轮对每个任务组：
                      ├─ prepare_execution：提交 bump 计数、回收队列排入空闲池
                      ├─ 轮询 pass：每个活跃任务跑一次 device_poll（解析/休眠/生成子任务/唤醒父任务）
                      └─ 压缩 pass：休眠任务移出活跃列表、保留新生成任务
                          └─ 回读（可选）：read_back_execution_states 统计活跃/空闲/休眠完成数
```

## 任务与任务池

### 槽位布局

[TaskPool::create_with_size](../../shader/task-graph/src/runtime/task_pool.rs#L26) 把任务池分配为一块 `[TaskType{i}]` 结构体数组，每个槽位五个字段：

```text
struct TaskType{i} {
  is_finished: u32,            // 轮询管线用，0/1
  payload: P,                  // 该任务组的 payload 类型（定义任务组时声明）
  state: S,                    // 任务状态结构体，由 DynamicTypeBuilder 动态构建
  parent_task_type_id: u32,    // 生成我的任务所在的组
  parent_task_index: u32,      // 生成我的任务在池中的索引
}
```

payload 类型在 `define_task_dyn` 时声明（[runtime/mod.rs:76](../../shader/task-graph/src/runtime/mod.rs#L76)），任务逻辑通过 `ctx.access_self_payload::<T>()`（[future_context.rs:145](../../shader/task-graph/src/runtime/future_context.rs#L145)）按自己知道的类型读写——payload 声明类型与 future 输出类型可以不同，这正是 wavefront 把"像素 id"、"追踪请求"、"命中上下文"装进不同任务组 payload 的方式。

### 状态机

任务状态常量定义在 [task_pool.rs:85](../../shader/task-graph/src/runtime/task_pool.rs#L85)：

| 常量 | 值 | 含义 |
| --- | --- | --- |
| `TASK_STATUE_FLAG_TASK_NOT_EXIST` | 0 | 槽位空闲（已回收或从未使用） |
| `TASK_STATUE_FLAG_NOT_FINISHED_WAKEN` | 1 | 已生成 / 已被唤醒，等待被轮询 |
| `TASK_STATUE_FLAG_GO_TO_SLEEP` | 2 | 本轮已轮询、尚未解析，仍留在活跃列表中（等待轮末压缩） |
| `TASK_STATUE_FLAG_NOT_FINISHED_SLEEP` | 3 | 已压缩出活跃列表的休眠任务 |
| `TASK_STATUE_FLAG_FINISHED` | 4 | 已解析，等待父任务读取结果并回收 |

转换路径（[task_pool.rs:99](../../shader/task-graph/src/runtime/task_pool.rs#L99) 的注释解释了 2 号状态的来历）：

```text
0 NOT_EXIST ──spawn_new_task_dyn──► 1 WAKEN ──轮询未解析──► 2 GO_TO_SLEEP ──轮末压缩──► 3 SLEEP
    ▲                                 │
    │ 回收（cleanup）                 │ 轮询解析
    └────────────── 4 FINISHED ◄──────┘

3 SLEEP ──wake_task_dyn（子任务解析后）──► 1 WAKEN
```

- `spawn_new_task_dyn`（[task_pool.rs:143](../../shader/task-graph/src/runtime/task_pool.rs#L143)）：从空闲池取索引，写入 payload、父任务引用，按 `fields_init` 初始化状态结构体各字段（无默认值的字段清零），状态置 1。活跃列表的登记由调用方（`TaskGroupDeviceInvocationInstance::spawn_new_task_dyn`，[task_group.rs:436](../../shader/task-graph/src/runtime/task_group.rs#L436)）负责：先从 `empty_index_pool` 弹一个空闲索引，再把这个索引 bump 进 `active_task_idx`。
- 轮询解析：状态置 4。无父任务（`parent_task_index == u32::MAX`）时当场回收（[task_group.rs:137](../../shader/task-graph/src/runtime/task_group.rs#L137)）；有父任务时通过 `switch_by(parent_task_type_id)` 找到父任务所在组的 spawner 调 `wake_task_dyn`（[task_group.rs:141](../../shader/task-graph/src/runtime/task_group.rs#L141)）。
- `wake_task_dyn`（[task_group.rs:454](../../shader/task-graph/src/runtime/task_group.rs#L454)）：状态置 1；只有"不在活跃列表中"（即状态不是 2）时才重新 bump 进活跃列表——状态 2 的任务本轮还在活跃列表里，不能重复登记。
- 回收：`cleanup_finished_task_state_and_payload`（[task_group.rs:495](../../shader/task-graph/src/runtime/task_group.rs#L495)）把任务索引写进 `new_removed_task_idx` 并把状态清 0；下一次 `prepare_execution` 把回收队列整体搬进空闲池（[task_group.rs:294](../../shader/task-graph/src/runtime/task_group.rs#L294)），槽位即可被复用。

## 执行引擎：一轮轮询做了什么

### 任务组的构建

`DeviceTaskGraphBuildSource::build`（[runtime/mod.rs:94](../../shader/task-graph/src/runtime/mod.rs#L94)）分三步：

- `TaskGroupExecutor::pre_build`（[task_group.rs:38](../../shader/task-graph/src/runtime/task_group.rs#L38)）：对每个任务组调用 `future.build_poll(&mut DeviceTaskSystemBuildCtx)` 生成调用实例。构建期上下文（[future_context.rs:7](../../shader/task-graph/src/runtime/future_context.rs#L7)）持有 `DynamicTypeBuilder`（状态结构体）与"本任务组依赖了哪些组"的登记表；future 里遇到 `TaskFuture::new(task_ty)` 就调 `get_or_create_task_group_instance`（[future_context.rs:90](../../shader/task-graph/src/runtime/future_context.rs#L90)），把依赖关系记进共享表并拿到一个**延迟解析的 spawner 句柄**（`TaskGroupDeviceInvocationInstanceLateResolved`，[future_context.rs:30](../../shader/task-graph/src/runtime/future_context.rs#L30)）——因为构建时各组的 spawner 绑定还没生成。
- 资源分配：`TaskGroupExecutorResource::create_with_size`（[task_group.rs:318](../../shader/task-graph/src/runtime/task_group.rs#L318)）按 `max_in_flight × capacity` 分配三个 bump 分配器（活跃索引、空闲池、回收队列）与任务池（多一个槽位给"默认任务"），随后 `init` pass（[task_group.rs:347](../../shader/task-graph/src/runtime/task_group.rs#L347)）把空闲池填满 1..N、在槽位 0 播一个零载荷的默认任务。
- `TaskGroupExecutor::build`（[task_group.rs:71](../../shader/task-graph/src/runtime/task_group.rs#L71)）：把依赖的（下游）任务组与本组的父依赖组的 spawner 绑定进本组轮询管线，解析所有延迟句柄，最后生成**轮询管线**。

### 轮询管线（核心）

轮询管线的 shader 逻辑（[task_group.rs:103](../../shader/task-graph/src/runtime/task_group.rs#L103)）每线程处理一个活跃任务：

```rust
// 线程 global id 可能超出活跃数量，越界线程固定轮询槽位 0（默认任务）
let task_index = active_idx
  .less_than(active_task_count.load())
  .select_branched(|| indices.index(active_idx).load(), || val(0));

let item = pool.rw_states(task_index);
state_to_resolve.resolve(item);          // 把任务的状态结构体指针喂给 DynamicTypeBuilder

let mut poll_ctx = DeviceTaskSystemPollCtx { self_task_idx: task_index, ... };
let poll_result = invocation.device_poll(&mut poll_ctx);   // 真正的任务逻辑

if_by(poll_ctx.is_fallback_task().not(), || {
  if_by(poll_result.is_resolved(), || {
    pool.rw_task_state(task_index).store(FINISHED);
    // 无父任务 → 回收；有父任务 → 按 parent_task_type_id 唤醒父任务
  })
  .else_by(|| pool.rw_task_state(task_index).store(GO_TO_SLEEP));
});
```

两个关键细节：

- **默认任务与 uniform control flow**：`is_fallback_task()`（[future_context.rs:133](../../shader/task-graph/src/runtime/future_context.rs#L133)）检查 `self_task_idx == 0`。槽位 0 在 `init` 时播种、永不进入活跃列表、永远保持 pending（`BaseFutureInvocation` 对 fallback 特判返回未解析，[future/mod.rs:60](../../shader/task-graph/src/future/mod.rs#L60)），轮询管线对 fallback 也跳过状态写入。它的存在是为了让越界线程仍然执行 `device_poll`——EDSL 依赖 uniform 控制流（如 `storage_barrier`），不能让一部分线程跳过整个轮询逻辑。
- **活跃列表**：`active_task_idx` 存的是"待轮询任务索引"的有序数组，轮询 pass 用间接派发，dispatch 规模由 `prepare_dispatch_size`（[bump_allocator.rs:64](../../shader/task-graph/src/bump_allocator.rs#L64)）从 `current_size` 实时算出。

### 轮末压缩

`use_compact_alive_tasks`（[task_group.rs:220](../../shader/task-graph/src/runtime/task_group.rs#L220)）先提交活跃列表计数（轮询过程中任务可能生成子任务，bump 计数要先折叠），然后对活跃索引数组跑一次流压缩（parallel-compute 的 `use_stream_compaction`），谓词 `ActiveTaskCompact`（[dispatch_compact.rs:43](../../shader/task-graph/src/runtime/dispatch_compact.rs#L43)）：

- 状态 2（GO_TO_SLEEP）→ 改写为 3（NOT_FINISHED_SLEEP）并剔除出活跃列表；
- 状态 1（WAKEN，即本轮新生成的子任务）→ 保留。

压缩结果拷回备用缓冲并 swap，随后一个小 pass 更新 `current_size`（[task_group.rs:259](../../shader/task-graph/src/runtime/task_group.rs#L259)）。这样"休眠任务不重复轮询"与"任务可自生成"两个约束同时满足。

### 宿主侧驱动

`DeviceTaskGraphExecutor::execute`（[runtime/mod.rs:419](../../shader/task-graph/src/runtime/mod.rs#L419)）按 `dispatch_round_count` 轮执行上面的流程；每一轮图被推进一层（任务解析 → 唤醒父任务 → 下一轮父任务被轮询）。宿主不知道图何时收敛，轮数是一个保守上限（wavefront 用 `execution_round_hint` 配置，默认 4，见 [shader/ray-tracing/src/api/pipeline.rs:12](../../shader/ray-tracing/src/api/pipeline.rs#L12)）。`read_back_execution_states`（[runtime/mod.rs:313](../../shader/task-graph/src/runtime/mod.rs#L313)）回读每组的三个计数：`wake_counts`（活跃数）、`empty_counts`（空闲池余量）、`sleep_or_finished_counts`（总容量 - 空闲 - 活跃，即休眠与已解析未回收的任务数）。测试用它断言每轮之后"该醒的醒、该睡的睡"。

`dispatch_allocate_init_task`（[runtime/mod.rs:249](../../shader/task-graph/src/runtime/mod.rs#L249)）是播种入口：一个 compute pass 让每个线程用 `TaskSpawnerInvocation::spawn_task(global_id, count)` 从全局 id 计算 payload 并生成任务（`TaskSpawner` trait 见 [runtime/mod.rs:177](../../shader/task-graph/src/runtime/mod.rs#L177)）。`dispatch_allocate_init_task_by_fn`（[runtime/mod.rs:207](../../shader/task-graph/src/runtime/mod.rs#L207)）是闭包便捷版本。

### 前后执行钩子

`set_task_before_execution_hook` / `set_task_after_execution_hook`（[runtime/mod.rs:186](../../shader/task-graph/src/runtime/mod.rs#L186)）在宿主侧、轮询 pass 前后执行。wavefront 用它按轮 reset 两个载荷 bump 分配器（见下文）。

## bump_allocator 模块：GPU 侧临时分配器

[DeviceBumpAllocationInstance](../../shader/task-graph/src/bump_allocator.rs#L4) 是任务图所有"动态数量"存储的底座：活跃任务索引列表、空闲池、回收队列、以及 wavefront 的未类型化用户载荷区，都是它。

### 两阶段分配模型

GPU 线程不能直接改 `current_size`（有数据竞争且派发规模依赖它），所以拆成两步：

- **阶段一（派发内）**：线程只对 `bump_size` 做 `atomic_add`（[bump_allocator.rs:248](../../shader/task-graph/src/bump_allocator.rs#L248)），得到自己分配区的起始偏移；越界检查基于"当前 `current_size` + 本次 bump"计算，返回 `(write_idx, in_bound)`。`bump_allocate_by` / `bump_deallocate`（[bump_allocator.rs:260](../../shader/task-graph/src/bump_allocator.rs#L260)、[bump_allocator.rs:307](../../shader/task-graph/src/bump_allocator.rs#L307)）在这个偏移上写/读元素。
- **阶段二（派发间）**：`commit_size`（[bump_allocator.rs:102](../../shader/task-graph/src/bump_allocator.rs#L102)）起一个单线程 pass，把 `bump_size` 折叠进 `current_size`（分配方向累加、释放方向累减，双向都钳位到数组边界），然后把 `bump_size` 清零。分配/释放方向不同（`previous_is_allocate` 参数），因为累加和累减的钳位逻辑不同。

注释强调同一派发内不能混用 allocate 与 deallocate 两种 bump——`commit_size` 不知道两次 bump 的正负如何抵消。

### 配套操作

- `prepare_dispatch_size`（[bump_allocator.rs:64](../../shader/task-graph/src/bump_allocator.rs#L64)）：把 `current_size` 转成 `DispatchIndirectArgsStorage`（取整到工作组大小的间接派发参数）。
- `drain_self_into_the_other`（[bump_allocator.rs:147](../../shader/task-graph/src/bump_allocator.rs#L147)）：把一个分配器的内容整体搬进另一个（如回收队列 → 空闲池）：先按源 `current_size` 间接派发逐元素拷到目标末尾，再一个单线程 pass 同时更新两个 `current_size`（目标累加、源清零）。这个 pass 要求两个分配器都已提交（committed）——这正是 `prepare_execution` 里先 commit 后 drain 的顺序（[task_group.rs:285](../../shader/task-graph/src/runtime/task_group.rs#L285)）。
- `reset`（[bump_allocator.rs:25](../../shader/task-graph/src/bump_allocator.rs#L25)）：清零 `current_size` 与 `bump_size`，供帧间/轮间复用。
- `debug_execution`（[bump_allocator.rs:46](../../shader/task-graph/src/bump_allocator.rs#L46)）：回读当前有效内容（按 `current_size` 截断）。

### 任务组里的三个实例

| 实例 | 方向 | 用途 |
| --- | --- | --- |
| `active_task_idx` | allocate | 待轮询任务索引的有序列表；轮询 pass 的派发规模与压缩的输入 |
| `empty_index_pool` | deallocate（栈式弹出） | 空闲槽位池：生成任务时弹一个索引，回收后经 `new_removed_task_idx` 排回 |
| `new_removed_task_idx` | allocate | 已回收任务索引的暂存队列，`prepare_execution` 时整体排入空闲池 |

注意空闲池用释放方向（`build_deallocator_shader`，[bump_allocator.rs:224](../../shader/task-graph/src/bump_allocator.rs#L224)）："弹出一个空闲索引"在实现上就是一次 bump_deallocate（从栈尾取），越界即池空。

## future 模块：任务的异步组合模型

### 两个 trait 的分工

- `ShaderFuture`（[future/mod.rs:68](../../shader/task-graph/src/future/mod.rs#L68)）是**任务定义**：`build_poll` 在构建期把任务逻辑编译成调用实例（并顺带声明任务状态结构体的字段）、`bind_input` 在派发期绑定输入、`required_poll_count` 只是预估轮询次数（真正能停多久由轮询驱动决定）。
- `ShaderFutureInvocation`（[future/mod.rs:36](../../shader/task-graph/src/future/mod.rs#L36)）是**任务逻辑**：`device_poll(ctx) -> ShaderPoll<Output>`，返回 `{ resolved: ShaderPtrOf<bool>, payload }`（[future/mod.rs:11](../../shader/task-graph/src/future/mod.rs#L11)）。`mark_resolved` 写入 resolved 标志。

trait 文档强调两条纪律：

- **必须 fused**：`device_poll` 可在任意时刻被多次调用（同一任务可能被反复唤醒），实现必须保证重复调用幂等——`BaseFutureInvocation`（[future/mod.rs:49](../../shader/task-graph/src/future/mod.rs#L49)）就是标准示范：用一个状态字段记录"已解析过"，第一次调用解析、之后恒为未解析；fallback 任务特判恒为未解析。
- **uniform 控制流**：`device_poll` 保证在 uniform 控制流内被调用，因此实现里可以安全使用 `storage_barrier`。

### 组合子

| 组合子 | 位置 | 语义 |
| --- | --- | --- |
| `map(f)` | [future/map.rs:3](../../shader/task-graph/src/future/map.rs#L3) | 上游解析后把输出变换成新类型 |
| `then(f, then_future)` | [future/then.rs:4](../../shader/task-graph/src/future/then.rs#L4) | 上游解析后，用 `create_then_invocation_instance(&输出, then实例, ctx)` 初始化下游 future 的抽象左值（如生成一个子任务并把句柄存入状态），然后轮询下游；一次轮询同时给出上下游的 payload 元组 |
| `then_spawn_task(f, task_ty)` | [future/mod.rs:137](../../shader/task-graph/src/future/mod.rs#L137) | `then` 的特化：下游是 `TaskFuture` |
| `into_dyn()` | [future/mod.rs:104](../../shader/task-graph/src/future/mod.rs#L104) | 擦除成 `DynShaderFuture<T>`（输出类型保留，调用实例擦成 boxed trait） |

组合子的实现都遵循同一结构：先轮询上游，`if_by(resolved)` 里做转换/初始化，再轮询下游，返回两者 payload。注意 `then` 的一次轮询里**上、下游各被轮询一次**，所以 `required_poll_count` 是相加关系（[future/then.rs:26](../../shader/task-graph/src/future/then.rs#L26)）。

### TaskFuture：跨组等待

`TaskFuture<T>`（[future/task.rs:2](../../shader/task-graph/src/future/task.rs#L2)）把一个任务组的子任务暴露成 future：构建时向 `DeviceTaskSystemBuildCtx` 登记依赖（`get_or_create_task_group_instance`），并把任务句柄存进状态字段（默认 `UN_INIT_TASK_HANDLE = u32::MAX - 1`）。轮询时（[future/task.rs:69](../../shader/task-graph/src/future/task.rs#L69)）：

- 句柄未初始化（`UN_INIT`）或已解析（`RESOLVED = u32::MAX`）→ 不轮询（`task_not_exist`）；
- 否则调 `spawner.poll_task::<T>`（[task_group.rs:471](../../shader/task-graph/src/runtime/task_group.rs#L471)）：子任务状态为 FINISHED 时读出其 payload、回收子任务槽位，返回 resolved；读到的结果写入输出左值并把句柄标记为 `RESOLVED`（"一旦解析就不能再轮询，因为状态已被回收"）。

`TaskFutureInvocationRightValue`（[future/task.rs:94](../../shader/task-graph/src/future/task.rs#L94)）实现 `ShaderAbstractLeftValue`——句柄本身就是一个可被 `then` 初始化、可存入状态字段的"抽象左值"（抽象左值机制见 [shader/api/src/abstract_load_store.rs:4](../../shader/api/src/abstract_load_store.rs#L4)）。

### 类型擦除与注册

任务组内所有 future 都以 `OpaqueTask`（[task_group.rs:3](../../shader/task-graph/src/runtime/task_group.rs#L3)）存放：`Box<dyn ShaderFuture<Output = Box<dyn Any>, Invocation = Box<dyn ShaderFutureInvocation<Output = Box<dyn Any>>>>`。`OpaqueTaskWrapper<T>`（[future/mod.rs:177](../../shader/task-graph/src/future/mod.rs#L177)）把任意 `ShaderFuture` 的输出装箱擦除——这是 `define_task_dyn`（[runtime/mod.rs:76](../../shader/task-graph/src/runtime/mod.rs#L76)）与 wavefront `create_task_graph` 的接缝。

`DeviceTaskSystemPollCtx`（[future_context.rs:124](../../shader/task-graph/src/runtime/future_context.rs#L124)）还带一个 `invocation_registry: AnyMap`——轮询期共享上下文的中转站：任务逻辑可以把任意 GPU 侧对象（payload 指针、spawner 实例）按类型注册/取出，供内层用户闭包使用（wavefront 用它把 `TracingCtx` 和追踪 spawner 注入用户着色器闭包，见下文）。

## dyn_ty_builder 模块：动态类型构建

任务的状态结构体在构建期才知道有哪些字段（由 future 的 `build_poll` 逐个声明），`DynamicTypeBuilder`（[dyn_ty_builder.rs:3](../../shader/task-graph/src/dyn_ty_builder.rs#L3)）解决"先有字段、后有结构体指针"的问题：

- `new_named` 创建一个具名空结构体（并强制塞入一个占位 u32 字段避免空结构体）。
- 每次 `create_or_reconstruct_inline_state`（[dyn_ty_builder.rs:45](../../shader/task-graph/src/dyn_ty_builder.rs#L45)）追加一个字段：记录 `fields_init`（默认值，`None` 表示 spawn 时清零），向 `ty` 追加字段类型，返回一个 `DeferResolvedStorageStructFieldNode`——一个"稍后解析"的抽象左值。
- 轮询管线里 `state_to_resolve.resolve(item)`（[task_group.rs:116](../../shader/task-graph/src/runtime/task_group.rs#L116)）把任务槽位的状态结构体指针填入 builder，所有字段节点第一次 `abstract_load/store` 时解析出对应字段指针并缓存（[dyn_ty_builder.rs:89](../../shader/task-graph/src/dyn_ty_builder.rs#L89)）。

`meta_info()`（`DynamicTypeMetaInfo { ty, fields_init }`，[dyn_ty_builder.rs:32](../../shader/task-graph/src/dyn_ty_builder.rs#L32)）是这份动态类型的"导出"：host 侧用它算任务池结构体的 stride 分配缓冲（[task_pool.rs:34](../../shader/task-graph/src/runtime/task_pool.rs#L34)），GPU 侧 `spawn_new_task_dyn` 用它按默认值/清零初始化每个新任务的状态字段（[task_pool.rs:165](../../shader/task-graph/src/runtime/task_pool.rs#L165)）——两份布局来自同一份元数据，保证一致。

## 下游衔接：wavefront 光线追踪

wavefront 后端把"光线追踪流水线"定义成一张任务图（[pipeline.rs:90](../../shader/ray-tracing/src/backend/wavefront_compute/pipeline.rs#L90) 的 `create_task_graph`）：

| 任务组 | payload | 作用 |
| --- | --- | --- |
| 0：trace 任务 | `TraceTaskSelfPayload`（[ctx.rs:6](../../shader/ray-tracing/src/backend/wavefront_compute/ctx.rs#L6)）：`sub_task_ty` / `sub_task_id` / `trace_call` | 核心追踪：遍历 BVH、按 SBT 命中/未命中分发子任务、回读用户载荷 |
| 1..：ray gen 任务 | `Vec3<u32>`（像素 id 占位） | 宿主播种；生成 trace 任务 |
| 其后：closest hit / miss 任务 | `create_composite_task_payload_desc`：`{ray 上下文, 用户 payload}`（[trace_task.rs:363](../../shader/ray-tracing/src/backend/wavefront_compute/trace_task.rs#L363)） | 命中/未命中着色；可以再生成 trace 任务（递归追踪） |

`graph.capacity = size`（瓦片 512×512，[pipeline.rs:206](../../shader/ray-tracing/src/backend/wavefront_compute/pipeline.rs#L206)），每个任务组的并发上限是 `max_in_flight × capacity`。`build` 时 `enable_buffer_combine` 由"每 stage storage buffer 上限 ≤ 256"判定（[pipeline.rs:53](../../shader/ray-tracing/src/backend/wavefront_compute/pipeline.rs#L53)）——资源紧张的平台把所有小缓冲合并进大缓冲（webgpu-virtual-typed-combine-buffer）。

**每帧执行**（[mod.rs:111](../../shader/ray-tracing/src/backend/wavefront_compute/mod.rs#L111) 的 `trace_ray`）：画面按 `rect_split_iter` 切成瓦片，每瓦片 `dispatch_allocate_init_task::<Vec3<u32>>` 用 `RangedTaskSpawner`（[mod.rs:199](../../shader/ray-tracing/src/backend/wavefront_compute/mod.rs#L199)，payload 从 uniform 里的 size/offset 换算像素坐标）播种 ray gen 任务，再 `execute` 若干轮推进图。

**trace 任务的轮询**（[trace_task.rs:149](../../shader/ray-tracing/src/backend/wavefront_compute/trace_task.rs#L149)）是一个多阶段状态机：

- 第一次轮询：`sub_task_id == TASK_NOT_SPAWNED`（`u32::MAX`）→ 走 BVH 遍历（`tlas_sys.traverse`），命中/未命中都通过 `spawn_dynamic`（[trace_task.rs:376](../../shader/ray-tracing/src/backend/wavefront_compute/trace_task.rs#L376)，按任务类型 switch 分发到对应任务组、把用户载荷从"未类型化载荷区"拷进子任务的组合 payload）生成子任务，`sub_task_ty/id` 存入自己的状态；生成失败标记 `TASK_SPAWNED_FAILED`。
- 子任务未生成 → 直接终止：把用户载荷从临时载荷区拷回**回读 bumper**（`payload_read_back_bumper`），`payload_ref` 指向回读位置。
- 子任务已生成 → `poll_dynamic`（[trace_task.rs:455](../../shader/ray-tracing/src/backend/wavefront_compute/trace_task.rs#L455)）：轮询子任务，解析后把子任务 payload 里的用户载荷拷进回读 bumper，更新 `payload_ref` 并标记终止（`has_terminated` 状态字段）。trace 任务一轮最多生成/等待一个子任务，多次被唤醒时走 `has_terminated` / `sub_task_id` 状态推进，天然 fused。

两个 bump 分配器的生命周期由执行钩子维护：轮询 trace 任务前 `payload_read_back_bumper.reset`，之后 `payload_bumper.reset`（[pipeline.rs:61](../../shader/ray-tracing/src/backend/wavefront_compute/pipeline.rs#L61)）——下一轮 ray gen 生成新 trace 任务时重新从零分配临时载荷区。

**用户侧两层实现**：用户着色器逻辑通过 `ShaderFutureProvider`（[api/operator.rs:3](../../shader/ray-tracing/src/api/operator.rs#L3)）提供 `build_device_future(AnyMap) -> DynShaderFuture`；`TraceOperator`（[api/operator.rs:22](../../shader/ray-tracing/src/api/operator.rs#L22)）把 future 实现与原生 RT 实现（`NativeRayTracingShaderBuilder`）统一成一份 DSL。例如 path tracing 的 ray gen（[scene/rendering/gpu-ray-tracing/src/feature/path_tracing/ray_gen.rs:13](../../scene/rendering/gpu-ray-tracing/src/feature/path_tracing/ray_gen.rs#L13)）：自定义 `ShaderFuture`，状态字段用 `ctx.make_state::<Node<u32>>()` 与 `create_or_reconstruct_inline_state_with_default` 声明（当前深度、吞吐率、累计 radiance），内部持有 `TracingFuture`（[trace_task.rs:681](../../shader/ray-tracing/src/backend/wavefront_compute/trace_task.rs#L681)——`TaskFuture<TraceTaskSelfPayload>` 的包装，解析后从回读 bumper 取出用户 payload），`required_poll_count` 按 `max_trace_depth` 逐层加码。

## 使用模板

### 模板一：最小任务图（宿主播种 + 定点执行）

```rust
let mut graph = DeviceTaskGraphBuildSource::default();
graph.capacity = 12;

let test_task = graph.define_task::<u32, _>(BaseShaderFuture::default().map(|_: (), _| {}), 2);

let mut graph_exe = graph.build(cx, false);

// 播种 3 个初始任务（payload 由闭包从全局 id 生成）
graph_exe.dispatch_allocate_init_task_by_fn(cx, 3, test_task, |_| val(0_u32));
cx.submit_recorded_work_and_continue();

let info = graph_exe.read_back_execution_states(cx).await;   // wake_counts == 3
graph_exe.execute(cx, 1, &graph);                            // 轮询一轮
let info = graph_exe.read_back_execution_states(cx).await;   // wake_counts == 0，全部完成
```

完整版见 [test.rs:2](../../shader/task-graph/src/test.rs#L2)（`test_simple_map`）、[test.rs:33](../../shader/task-graph/src/test.rs#L33)（`then_spawn_task` 父子两轮推进，是理解"子任务解析 → 唤醒父任务 → 下一轮父任务再轮询"的最小样例）、[test.rs:142](../../shader/task-graph/src/test.rs#L142)（任务自生成自己的递归样例，可观察到每轮 `sleep_or_finished_counts` 递增）。

### 模板二：任务组间依赖（then_spawn_task）

```rust
let test_task2 = graph.define_task::<u32, _>(
  BaseShaderFuture::default()
    .then(
      |_: &(), then, cx| {
        then.spawner
          .spawn_new_task(val(0_u32), cx.generate_self_as_parent())
          .unwrap()
      },
      TaskFuture::<u32>::new(test_task as usize),
    )
    .map(|_, _| {}),
  2,
);
```

`cx.generate_self_as_parent()`（[future_context.rs:136](../../shader/task-graph/src/runtime/future_context.rs#L136)）让子任务的父引用指向当前任务——这是任务图"边"的构造方式：test_task2 的任务在第一次轮询时生成 test_task 子任务并等待；子任务下一轮解析并唤醒父任务；再下一轮父任务读到结果后解析。

### 模板三：自定义任务逻辑

实现 `ShaderFuture`（构建 + 绑定）+ `ShaderFutureInvocation`（`device_poll`），状态用 `make_state` 声明，payload 用 `ctx.access_self_payload::<T>()` 读写。`PTRayGen`（[ray_gen.rs:13](../../scene/rendering/gpu-ray-tracing/src/feature/path_tracing/ray_gen.rs#L13)）与 `TraceTaskImpl`（[trace_task.rs:46](../../shader/ray-tracing/src/backend/wavefront_compute/trace_task.rs#L46)）是两个完整范例。自定义 future 的 `bind_input` 必须把输入绑定进 `DeviceTaskSystemBindCtx`（Deref 到 `BindingBuilder`，[future_context.rs:106](../../shader/task-graph/src/runtime/future_context.rs#L106)），与 `build_poll` 里 `bind_by` 的顺序一致。

## 约束与注意点

- **容量是静态的**：每任务组 `max_in_flight × capacity` 个槽位 + 1 个默认任务槽；任务数超过时 `bump_allocate` 返回失败（`shader_assert` 报警）。宿主播种时把 `dispatch_size` 当上限，播种数量不能超过容量。
- **一轮一个活跃子任务**：当前组合子（`then` 链）保证"生成子任务与首次轮询子任务不在同一轮"，父任务在子任务解析前已被压缩出活跃列表——这是任务图能稳定推进（无同轮重复唤醒）的结构性前提。自定义 `device_poll` 若在同一轮生成并立即轮询多个子任务，需要自行保证唤醒去重。
- **`device_poll` 必须 fused**：同一任务会被反复唤醒轮询，状态推进要放在任务状态字段里（`has_terminated`、`sub_task_id` 模式），不能假设"轮询一次就结束"。
- **轮询管线内所有线程都执行 `device_poll`**：越界线程轮询槽位 0（默认任务）。自定义 future 要像 `BaseFutureInvocation` 一样对 `is_fallback_task()` 特判，或保证零值状态下的轮询无害（wavefront trace 任务靠 sentinel 值 `u32::MAX` / `u32::MAX - 1` 与任务类型范围从 1 开始，零值 `sub_task_id=0` 不会命中任何分发分支）。
- **`TaskSpawner` 的管线哈希**：`dispatch_allocate_init_task_by_fn` 的闭包如果捕获了影响 shader 的值（如偏移量），哈希只覆盖闭包类型，同一调用点的不同捕获值会命中陈旧管线——要么用 uniform 传值（wavefront 的 `RangedTaskSpawner` 模式），要么保证闭包无捕获。
- **bump 分配器同一派发内不能混合 allocate 与 deallocate**。
- **`required_poll_count` 只是预估**：真正决定推进轮数的是 `execute` 的 `dispatch_round_count`（wavefront 默认 4），轮数不足时任务停留在休眠状态，可用 `read_back_execution_states` 观测。

## 延伸阅读

- 运行时核心：[runtime/mod.rs](../../shader/task-graph/src/runtime/mod.rs)（宿主驱动）、[runtime/task_group.rs](../../shader/task-graph/src/runtime/task_group.rs)（轮询管线与压缩）、[runtime/task_pool.rs](../../shader/task-graph/src/runtime/task_pool.rs)（任务槽位与状态机）、[runtime/future_context.rs](../../shader/task-graph/src/runtime/future_context.rs)（三个构建/绑定/轮询上下文）
- 异步模型：[future/mod.rs](../../shader/task-graph/src/future/mod.rs)、[future/task.rs](../../shader/task-graph/src/future/task.rs)、[future/then.rs](../../shader/task-graph/src/future/then.rs)
- 动态类型：[dyn_ty_builder.rs](../../shader/task-graph/src/dyn_ty_builder.rs)；抽象左值机制：[shader/api/src/abstract_load_store.rs:4](../../shader/api/src/abstract_load_store.rs#L4)
- GPU 单元测试：[test.rs](../../shader/task-graph/src/test.rs)（`gpu_cx!` 环境 + `debug_execution` 回读任务池原始字节，参考 [shader-edsl-compute-zh.md](skill-translation/shader-edsl-compute-zh.md) 的 GPU 单元测试模式）
- wavefront 集成：[shader/ray-tracing/src/backend/wavefront_compute/pipeline.rs](../../shader/ray-tracing/src/backend/wavefront_compute/pipeline.rs)、[trace_task.rs](../../shader/ray-tracing/src/backend/wavefront_compute/trace_task.rs)、[ctx.rs](../../shader/ray-tracing/src/backend/wavefront_compute/ctx.rs)
- 运行时自身的并行原语（流压缩、间接派发）：[parallel-compute-primitives-guide.md](parallel-compute-primitives-guide.md)
- 抽象左值（`ShaderAbstractLeftValue` / `BoxedShaderLoadStore`）与组合子相关 EDSL 细节：[skill-translation/shader-edsl-core-zh.md](skill-translation/shader-edsl-core-zh.md)
