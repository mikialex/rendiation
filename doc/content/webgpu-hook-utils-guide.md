# Rendiation GPU Hook 基建指南（platform/graphics/webgpu-hook-utils）

本文梳理 [platform/graphics/webgpu-hook-utils](../../platform/graphics/webgpu-hook-utils/src/lib.rs) 的实现：这是整个渲染侧共享的"数据库变化 → GPU 资源维护"底座。`QueryGPUHookCx` 的两阶段 GPU 维护（Update / CreateRender）是 [application/viewer-content/src/rendering_root.rs](../../application/viewer-content/src/rendering_root.rs) 帧时序的基础；`sparse_buffer_writes` / `sparse_update_storage_buffer` / `GrowableRangeAllocator` 封装则是材质、网格、灯光、扩展模型等一切"实体按分配索引常驻 GPU 槽位"数据的写入通道。本文同时覆盖被下游密集引用的 `use_result_ext`、`use_db_device_foreign_key`、`use_range_allocated_device_buffers`、`use_multi_access_gpu` 等工具与它们的 trait 抽象体系。

## 前置阅读

本模块是 hook 运行时（状态记忆）与增量查询（变化计算）在 GPU 资源层的宿主，建议先了解：

| 文档 | 内容 |
| --- | --- |
| [hooks-guide.md](hooks-guide.md) | hook 运行时：FunctionMemory、scope、动态/静态阶段（`use_plain_state` 等状态 API 的来源） |
| [query-hook-guide.md](query-hook-guide.md) | 两阶段执行模型（spawn/resolve）、UseResult 四态、共享计算、task pool（本模块的 Update/CreateRender 就是它的 GPU 特化） |
| [skill-translation/database-schema-zh.md](skill-translation/database-schema-zh.md) | 实体/组件/外键、实体分配索引（alloc index）、global_database 的读视图 |
| [skill-translation/shader-edsl-binding-and-typed-container-zh.md](skill-translation/shader-edsl-binding-and-typed-container-zh.md) | GPU 资源容器与绑定（storage buffer / uniform buffer 的 bind 语义） |
| [skill-translation/shader-edsl-compute-zh.md](skill-translation/shader-edsl-compute-zh.md) | compute 管线构建（稀疏写的 GPU 侧实现用 compute 派发完成） |
| [batch-extractor-guide.md](batch-extractor-guide.md) | 本模块的下游使用者之一：批提取器如何用稀疏写与范围分配器维护 id 池 |

阅读源码时可先看 [platform/graphics/webgpu/src/resource/buffer/](../../platform/graphics/webgpu/src/resource/buffer/) 的缓冲资源层：`AbstractBuffer` / `AbstractStorageAllocator`（[abstract_resource.rs](../../platform/graphics/webgpu/src/resource/buffer/abstract_resource.rs)）与 `GPULinearStorage` / `ResizableGPUBuffer` 系列（[linear_buffer_array/mod.rs](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/mod.rs)）是本文所有缓冲结构的地基。

## 模式概览

整个 crate 回答一个核心问题：**数据库里"某实体的一行数据变了"，如何以最低代价反映到 GPU 上的一个常驻存储缓冲槽位**。它给出的答案是"两阶段 + 稀疏"：

- **两阶段 GPU 维护**：`QueryGPUHookCx` 把一帧的 GPU 资源维护拆成 `Update`（spawn 阶段：收集变化、做可并行的宿主侧计算、把异步任务装进 task pool）与 `CreateRender`（resolve 阶段：取回任务结果，在帧 encoder 上执行 GPU 写入、输出渲染器对象）两个阶段。同一份 hook 函数在两个阶段各执行一遍，靠 `cx.stage` 区分行为。
- **稀疏写入**：变化被映射为"实体分配索引 + 字段字节偏移 → 新值"的打包写源 `SparseBufferWritesSource`，render 阶段用一个 compute 派发把所有碎片写一次性复制进目标缓冲——改一个字段只写一个槽的一个字段，绝不整块重写。
- **可增长存储缓冲**：`SparseUpdateStorageBuffer` 把"会按数据库表容量增长的只读存储缓冲"（`CustomGrowBehaviorMaintainer<ResizableGPUBuffer<...>>`）与"每帧的稀疏写收集器"绑在一起，`use_update` 负责两阶段的接缝。
- **范围分配器封装**：`GrowableRangeAllocator`（[utility/growable-range-allocator](../../utility/growable-range-allocator/src/lib.rs)）为"每个 key 一段 GPU 连续区域"的场景提供批量分配/释放/重定位，`BatchAllocateResult` 把每次 update 产出的"新写、搬迁、移除、扩容"四类变更一次性交给 GPU 层落地。
- **trait 抽象**：`DataChangeGPUExt` / `DataChangeGPUExtForUseResult` 是"变化 → GPU 写入"的统一入口，下游任何 `DataChanges` 或 `UseResult<DataChanges>` 都可以直接 `.update_storage_array(...)` / `.update_uniforms(...)` 挂进对应资源，无需关心两阶段细节。

## 核心概念

| 概念 | 定义位置 | 说明 |
| --- | --- | --- |
| `QueryGPUHookCx` | [hook.rs:9](../../platform/graphics/webgpu-hook-utils/src/hook.rs#L9) | GPU 资源维护的 hook 宿主：FunctionMemory + GPU + 存储分配器 + 共享上下文 + 阶段 |
| `GPUQueryHookStage` | [hook.rs:20](../../platform/graphics/webgpu-hook-utils/src/hook.rs#L20) | `Update`（spawn：任务池/派发器/inspector）与 `CreateRender`（resolve：任务结果/帧 encoder） |
| `QueryGPUHookFeatureCx` | [hook.rs:3](../../platform/graphics/webgpu-hook-utils/src/hook.rs#L3) | `use_state_with_features` 的 init 参数：gpu / shared_ctx / storage_allocator |
| `SparseBufferWritesSource` | [sparse_buffer_writes.rs:4](../../platform/graphics/webgpu-hook-utils/src/sparse_buffer_writes.rs#L4) | 稀疏写源：打包数据 + (源偏移, 长度, 目标偏移) 三元组表 |
| `SparseUpdateStorageBuffer<T>` | [sparse_update_storage_buffer.rs:11](../../platform/graphics/webgpu-hook-utils/src/sparse_update_storage_buffer.rs#L11) | 可增长只读存储缓冲 + 每帧稀疏写收集器 |
| `SparseUpdateStorageWithHostBuffer<T>` | [sparse_update_storage_buffer.rs:76](../../platform/graphics/webgpu-hook-utils/src/sparse_update_storage_buffer.rs#L76) | 上述缓冲 + 宿主 Vec 备份（host-driven 路径读回数据用） |
| `GrowableRangeAllocator<K>` | [utility/growable-range-allocator/src/lib.rs:7](../../utility/growable-range-allocator/src/lib.rs#L7) | 增长式范围分配器：K → (size, offset)，批量 update 产出四类变更 |
| `BatchAllocateResult<K>` | [growable-range-allocator/src/lib.rs:267](../../utility/growable-range-allocator/src/lib.rs#L267) | 一次批量分配的结果：removed / failed / data_movements / new_data_to_write / resize_to |
| `BatchAllocateResultShared<K>` | [allocator.rs:9](../../platform/graphics/webgpu-hook-utils/src/allocator.rs#L9) | 上述结果的 Arc 包装 + 按 item 换算成字节的 GPU 落地辅助（apply_resize / iter_data_movements） |
| `RangeAllocateBufferCollector<K>` | [allocator.rs:99](../../platform/graphics/webgpu-hook-utils/src/allocator.rs#L99) | 待写数据的收集器：小写合并成大包、大写按 key 单独存 |
| `RangeAllocateBufferUpdates<K>` | [allocator.rs:211](../../platform/graphics/webgpu-hook-utils/src/allocator.rs#L211) | 范围分配场景的"待落地"变更包（稀疏写 + 分配结果），render 阶段消费 |
| `DEVICE_RANGE_ALLOCATE_FAIL_MARKER` | [allocator.rs:70](../../platform/graphics/webgpu-hook-utils/src/allocator.rs#L70) | `u32::MAX`，分配失败标记（宿主侧据此跳过实体） |
| `DataChangeGPUExt` / `DataChangeGPUExtForUseResult` | [use_result_ext.rs:26](../../platform/graphics/webgpu-hook-utils/src/use_result_ext.rs#L26) | 变化 → GPU 写入的统一 trait：uniform / uniform array / storage array 三条通道 |
| `UniformBufferCollection<K, V>` | [use_result_ext.rs:24](../../platform/graphics/webgpu-hook-utils/src/use_result_ext.rs#L24) | 每 key 一个独立 uniform buffer 的集合（GLES 路径用） |
| `BindingArrayMaintainer<V>` | [binding_array.rs:3](../../platform/graphics/webgpu-hook-utils/src/binding_array.rs#L3) | bindless binding array 的整表重建维护器 |
| `MultiAccessGPUData` | [multi_access.rs:171](../../platform/graphics/webgpu-hook-utils/src/multi_access.rs#L171) | one→many 引用的 GPU 侧表达：meta（范围表）+ indices（索引池）双缓冲 |
| `AbstractStorageAllocator` | [webgpu/src/resource/buffer/abstract_resource.rs:3](../../platform/graphics/webgpu/src/resource/buffer/abstract_resource.rs#L3) | 存储缓冲分配抽象（可换成 texture-as-buffer 等实现），`allocate_readonly` 经它产生缓冲 |
| `ResizableGPUBuffer<T>` | [webgpu/src/resource/buffer/linear_buffer_array/gpu_raw.rs:3](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/gpu_raw.rs#L3) | 可 resize 的 GPU 线性存储（新大小按 item 计数） |

## QueryGPUHookCx：两阶段 GPU 维护模型

### 结构组成

[hook.rs:9](../../platform/graphics/webgpu-hook-utils/src/hook.rs#L9) 的 `QueryGPUHookCx` 在通用 hook 上下文上叠加了 GPU 能力：

```rust
pub struct QueryGPUHookCx<'a> {
  pub memory: &'a mut FunctionMemory,          // hook 状态记忆（见 hooks-guide）
  pub gpu: &'a GPU,
  pub storage_allocator: Box<dyn AbstractStorageAllocator>, // 缓冲分配策略
  pub shared_ctx: &'a mut SharedHooksCx,       // 共享计算上下文（见 query-hook-guide）
  pub stage: GPUQueryHookStage<'a>,            // 当前阶段
  pub waker: Waker,
  pub dyn_cx: &'a mut DynCx,
}
```

它同时实现了三层 trait：

- `HooksCxLike`（[hook.rs:34](../../platform/graphics/webgpu-hook-utils/src/hook.rs#L34)）：`is_dynamic_stage` 只在 `Update` 阶段为 true（只有 spawn 阶段允许创建新状态形状）；`flush` 只在 `Update` 阶段执行——GPU 资源的清理（如删掉的 buffer）被推迟到 spawn 阶段统一做。
- `InspectableCx`（[hook.rs:61](../../platform/graphics/webgpu-hook-utils/src/hook.rs#L61)）：`if_inspect` 在 `Update` 阶段把 `Inspector` 暴露给资源钩子，用于上报内存占用（`label_device_memory_usage` 等）。
- `QueryHookCxLike`（[hook.rs:210](../../platform/graphics/webgpu-hook-utils/src/hook.rs#L210)）：把 `GPUQueryHookStage` 映射成 `QueryHookStage::SpawnTask` / `ResolveTask`，使所有 query-hook 的组合子（`map_spawn_stage_in_thread`、`use_assure_result`、共享计算……）原样可用。

### Update / CreateRender 两阶段的职责划分

[hook.rs:20](../../platform/graphics/webgpu-hook-utils/src/hook.rs#L20) 的两个阶段变体：

| | `Update` | `CreateRender` |
| --- | --- | --- |
| 对应 query-hook 阶段 | SpawnTask | ResolveTask |
| 携带 | `task_pool`（AsyncTaskPool）、`spawner`（rayon 线程池）、`immediate_results`、`inspector` | `task`（TaskPoolResultCx）、`encoder`（帧 GPUCommandEncoder） |
| 可以做什么 | 声明订阅（use_changes / use_dual_query）、同步计算、把未来安装进 task pool、向收集器 push 写任务、resize 缓冲（单独 encoder 立即提交） | 按 token 取回任务结果、在帧 encoder 上执行 GPU 写入、组装并返回渲染器对象 |
| 动态阶段 | 是（可创建新状态） | 否（只能按既定形状访问） |
| 产出 | `UseResult::SpawnStageFuture / SpawnStageReady` | `UseResult::ResolveStageReady` |

职责划分的动机：**宿主侧计算（分配、打包、排序）与 GPU 侧写入（复制、重定位）分离**。宿主计算可以放进 rayon 线程池并行；GPU 写入必须落在帧 encoder 上、与渲染 pass 保持提交顺序。`CreateRender` 阶段结束时得到的渲染器对象（如 `IndirectSceneRenderer`）直接交给后续的帧组装使用。

### 驱动循环：RenderingRoot

真正的驱动在 [application/viewer-content/src/rendering_root.rs:77](../../application/viewer-content/src/rendering_root.rs#L77) 的 `draw_canvas`，每帧的时序是：

```text
FrameCtx::new(...) 创建帧上下文（含 encoder）
  ├─ 第一次 execute：构造 QueryGPUHookCx，stage = GPUQueryHookStage::Update { ... }
  │    └─ rendering.use_viewer_scene_renderer(cx, ...)   // 同一份函数，spawn 行为
  ├─ pollster::block_on(pool.all_async_task_done())       // 等待全部任务完成
  │    └─ 结果并入 task_pool_result
  ├─ 第二次 execute：构造 QueryGPUHookCx，stage = GPUQueryHookStage::CreateRender { task, encoder }
  │    └─ rendering.use_viewer_scene_renderer(cx, ...)    // 同一份函数，resolve 行为
  │         ├─ 取回任务结果，写 GPU 资源（稀疏写、重定位写）
  │         └─ when_render(...) 内组装渲染器实例
  └─ rendering.render(...)  // 真正的渲染 pass（消费渲染器对象）
```

两个阶段之间 `shared_ctx.reset_visiting()` 需要被调用两次（每次 execute 前），`render_resource_memory`（跨帧的 FunctionMemory）贯穿两个阶段——所以**同一调用点在两阶段访问的是同一份状态**，这是"spawn 阶段存 token、resolve 阶段按 token 取回"能成立的根基。每轮帧结束后的 `task_pool_result` 被挪到另一个任务里 drop（避免拖慢主线程）。

注意 `use_viewer_scene_renderer` 这类函数在**两个阶段各执行一遍**，因此它的返回值是 `Option<...>`：`when_render` 只允许在 `CreateRender` 阶段产生实际值，`Update` 阶段返回 `None`。

### 状态与资源 API

hook 的通用状态 API 都可用，另有 GPU 特化的几个：

- `use_state` / `use_state_init` / `use_plain_state`：普通状态（跨帧持久，见 hooks-guide）。
- `use_state_with_features`（[hook.rs:75](../../platform/graphics/webgpu-hook-utils/src/hook.rs#L75)）：init 闭包收到 `QueryGPUHookFeatureCx`（gpu / shared_ctx / storage_allocator），用于"初始化就需要 GPU"的状态；`use_gpu_init`（[hook.rs:114](../../platform/graphics/webgpu-hook-utils/src/hook.rs#L114)）是它的常见简写——**所有 GPU 资源的创建几乎都走 `use_gpu_init`**，保证资源只创建一次、跨帧持有。
- `use_sharable_plain_state`：`Arc<RwLock<T>>` 状态，供跨线程（spawn 阶段 worker）共享，范围分配器就是这样被 worker 线程写、render 阶段读的。
- `when_render(f)` / `is_in_render`（[hook.rs:191](../../platform/graphics/webgpu-hook-utils/src/hook.rs#L191)）：阶段门控——`CreateRender` 阶段执行 f 并返回 `Some(x)`，否则 `None`。**渲染器对象的组装必须包在这里**，否则 `Update` 阶段也会产生无意义的对象。
- `use_storage_buffer` / `use_storage_buffer_with_host_backup`（[hook.rs:139](../../platform/graphics/webgpu-hook-utils/src/hook.rs#L139)）：创建 `SparseUpdateStorageBuffer`（见下文），并在 `Update` 阶段重置其收集器。
- `use_uniform_buffers` / `use_uniform_array_buffers`（[hook.rs:123](../../platform/graphics/webgpu-hook-utils/src/hook.rs#L123)）：uniform 侧的资源。

### 状态生命周期与清理

`QueryGPUHookDropCx`（[hook.rs:204](../../platform/graphics/webgpu-hook-utils/src/hook.rs#L204)）是 `CanCleanUpFrom` 的 drop 上下文。`flush` 只在 `Update` 阶段真正清理本轮未访问的子作用域；跨帧记忆在 `RenderingRoot::cleanup`（[rendering_root.rs:43](../../application/viewer-content/src/rendering_root.rs#L43)）里对 `render_resource_memory` 整体 `cleanup`——所有 GPU 资源状态（buffer、绑定组等）随 Cx 退出统一销毁。

## 数据变化到 GPU 写入：DataChangeGPUExt trait 体系

[use_result_ext.rs](../../platform/graphics/webgpu-hook-utils/src/use_result_ext.rs) 定义"增量变化 → GPU 资源"的统一接口。核心是三条写入通道：

| 通道 | 资源形态 | 适用 | 实现位置 |
| --- | --- | --- | --- |
| `update_uniforms` | `UniformBufferCollection<K, V>`：每 key 一个独立 uniform buffer | GLES/host 路径逐实体绑定 | [use_result_ext.rs:26](../../platform/graphics/webgpu-hook-utils/src/use_result_ext.rs#L26) |
| `update_uniform_array` | `UniformArray<U, N>`：一个数组 uniform，按 `alloc_index` 寻址 | 固定容量、均匀布局的 host 路径数据 | 同上 |
| `update_storage_array` | `SparseUpdateStorageBuffer<U>`：存储缓冲，按 `alloc_index` 寻址 | indirect 路径的常驻 GPU 槽位数据（主要通道） | [use_result_ext.rs:43](../../platform/graphics/webgpu-hook-utils/src/use_result_ext.rs#L43) |

两个 trait 分属不同的输入形态：

- `DataChangeGPUExt<K>`：直接消费 `DataChanges<Key = K>`（同步数据）。blanket impl（[use_result_ext.rs:219](../../platform/graphics/webgpu-hook-utils/src/use_result_ext.rs#L219)）对任意 `DataChanges` 生效，同步更新 uniform 集合（remove 从 map 删除、insert 懒建 buffer 后 `write_at`）。
- `DataChangeGPUExtForUseResult<K>`：消费 `UseResult<T>`（异步数据）。`update_storage_array` / `update_storage_array_with_host` 在这里实现——因为稀疏写必须走两阶段。

### update_storage_array：spawn 收集、render 落地

以 [world_matrix.rs:15](../../scene/rendering/gpu-base/src/world_matrix.rs#L15) 的标准套路为例：

```rust
let (cx, storage) = cx.use_storage_buffer("scene model world mat", 128, u32::MAX);
cx.use_shared_dual_query(GlobalSceneModelWorldMatrix) // UseResult<DualQueryLike>
  .into_delta_change()
  .map(|v| v.collective_map(|mat| /* 转成 std430 布局 */))
  .update_storage_array(cx, storage, 0);              // 字段字节偏移 = 0

storage.use_max_item_count_by_db_entity::<SceneModelEntity>(cx); // 按表容量预增长
storage.use_update(cx);                                // 两阶段收口
```

`update_gpu_buffer_array_raw`（[use_result_ext.rs:148](../../platform/graphics/webgpu-hook-utils/src/use_result_ext.rs#L148)）是 `update_storage_array` 的底层实现，行为分阶段：

- `Update` 阶段：把变化值逐条转成 `(目标偏移 = alloc_index * item_byte_size + field_byte_offset, 数据)` 的 `collect_write` 调用，打包成一个 `SparseBufferWritesSource` 的 future，**push 进 storage 的收集器**（`SparseUpdateCollector`）。收集器里所有 future 之后会被 `use_update` 一起 join。
- `CreateRender` 阶段：直接 return（写入由 `use_update` 统一执行）。若变化链是 `ResolveStageReady`（典型是经过 `use_assure_result` 的链），debug 构建下检查"spawn 阶段是否预备过"，没有则 panic（"storage array update must prepared in spawn stage"）。

关键规则由此而来：**`update_storage_array` 必须在 spawn 阶段被调用（把写任务塞进收集器），不能只在 render 阶段调用**。这就是为什么材质存储的代码（[material/mr.rs:6](../../scene/rendering/gpu-indirect/src/material/mr.rs#L6)）把 `use_changes(...).update_storage_array(...)` 平铺在函数体里、而不是包在 `when_render` 里。

`update_storage_array_with_host` 走完全相同的收集通道，只是目标换成带宿主备份的缓冲（见下）。

## 稀疏写入机制：SparseBufferWritesSource

[sparse_buffer_writes.rs:4](../../platform/graphics/webgpu-hook-utils/src/sparse_buffer_writes.rs#L4) 是整个稀疏机制的数据载体：

```rust
pub struct SparseBufferWritesSource {
  pub data_to_write: Vec<u8>,   // 所有碎片数据顺序拼接
  pub offset_size: Vec<u32>,    // 每 3 个 u32 一组：(源偏移, 长度, 目标偏移)，全部以 u32(4字节) 为单位
}
```

- `collect_write(data, write_offset_in_bytes)`（[sparse_buffer_writes.rs:54](../../platform/graphics/webgpu-hook-utils/src/sparse_buffer_writes.rs#L54)）：把一段数据追加进 `data_to_write`，记录"源偏移、长度、目标偏移"。两个入参都断言 4 字节对齐——GPU 最小寻址单元是 u32。
- `merge`（[sparse_buffer_writes.rs:40](../../platform/graphics/webgpu-hook-utils/src/sparse_buffer_writes.rs#L40)）：把另一个写源拼到自己后面，重映射源偏移。多个写任务的结果在 worker 线程里用 merge 合并成一个，减少 render 阶段的派发次数。
- `iter_updates`（[sparse_buffer_writes.rs:24](../../platform/graphics/webgpu-hook-utils/src/sparse_buffer_writes.rs#L24)）：把三元组解包成 `(目标字节偏移, 数据切片)`，供 queue 直写路径使用。
- 不变量：**同一写源内的目标区间不允许重叠**（注释明确约定；`extra-checks` feature 下用 rangemap 校验并 panic，见 [sparse_buffer_writes.rs:115](../../platform/graphics/webgpu-hook-utils/src/sparse_buffer_writes.rs#L115)）。

### 两种落地方式

`write_abstract`（[sparse_buffer_writes.rs:73](../../platform/graphics/webgpu-hook-utils/src/sparse_buffer_writes.rs#L73)）按目标缓冲的形态二选一：

- 目标是真 GPU buffer（`get_gpu_buffer_view` 返回 Some）：走 **compute 复制**（`write`，[sparse_buffer_writes.rs:95](../../platform/graphics/webgpu-hook-utils/src/sparse_buffer_writes.rs#L95)）。`data_to_write` 与 `offset_size` 各创建一个只读 storage buffer，`target_buffer` 绑定为可写 storage buffer，派发一个计算管线：每个线程负责一组三元组，`loop_by` 逐 u32 从源复制到目标。管线哈希是 `SparseBufferWrite` 标记类型，走管线缓存。workgroup 宽度取设备 `max_compute_invocations_per_workgroup`，派发数 = `copy_count.div_ceil(workgroup_width)`。全部在调用方传入的帧 encoder 上执行，与后续渲染 pass 同一次提交。
- 目标是"以纹理模拟的缓冲"（`get_gpu_buffer_view` 返回 None）：退化为 `queue.write_buffer` 逐段直写。

用 compute 而不是 queue 直写的原因：直写要等 queue 排队、且多次调用之间没有合并；compute 路径把全部碎片写合并成**一次派发、一次绑定组设置**，且与帧编码同序，避免额外的同步点。

## 稀疏更新存储缓冲：SparseUpdateStorageBuffer

[sparse_update_storage_buffer.rs:11](../../platform/graphics/webgpu-hook-utils/src/sparse_update_storage_buffer.rs#L11) 把缓冲本身与每帧的收集器绑定：

```rust
type SparseStorageBufferRaw<T> =
  CustomGrowBehaviorMaintainer<ResizableGPUBuffer<AbstractReadonlyStorageBuffer<[T]>>>;

pub struct SparseUpdateStorageBuffer<T> {
  pub buffer: SparseStorageBufferRaw<T>,
  pub collector: Option<SparseUpdateCollector>, // Vec<Pin<FrameBox<dyn Future<Output = SparseBufferWritesSource>>>>
}
```

缓冲的三层组合（对应 [linear_buffer_array/mod.rs](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/mod.rs) 的组合子）：

- `AbstractReadonlyStorageBuffer<[T]>`：经 `AbstractStorageAllocator::allocate_readonly` 创建的类型化只读存储缓冲（分配策略可插拔，比如换成 texture-as-buffer）。
- `ResizableGPUBuffer`（`with_direct_resize`）：可 resize（新大小按 item 计数，resize 用独立 encoder 提交并拷贝旧内容）。
- `CustomGrowBehaviorMaintainer`（`with_default_grow_behavior(max)`，[grow_behavior.rs:11](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/grow_behavior.rs#L11)）：插入增长策略——`grow_at_least(required)` 时按 `max(current * 2, required, max)` 扩容；`max_item_count = u32::MAX` 表示不设上限。

对外 API：

- `use_max_item_count_by_db_entity<E>`（[sparse_update_storage_buffer.rs:51](../../platform/graphics/webgpu-hook-utils/src/sparse_update_storage_buffer.rs#L51)）：读数据库实体表容量并 `grow_at_least`，让缓冲与表容量同步增长（实体分配索引是稠密递增的，槽位下标 = alloc_index）。
- `use_update`（[sparse_update_storage_buffer.rs:67](../../platform/graphics/webgpu-hook-utils/src/sparse_update_storage_buffer.rs#L67)）：两阶段收口，实现见 `use_update_impl`（[sparse_update_storage_buffer.rs:139](../../platform/graphics/webgpu-hook-utils/src/sparse_update_storage_buffer.rs#L139)）。

`use_update_impl` 的流程：

```text
Update 阶段：
  collector.take()（每帧的收集器此刻被取走；再调一次 use_update 会 panic）
  ├─ 收集器为空 → token 记 u32::MAX，返回
  └─ 非空 → 构造 future：join_all 等全部写任务 → 多个结果 merge 成一个
            → spawner.spawn_task 再包一层（合并本身也可并行）
            → task_pool.install_task，token = 任务 id
CreateRender 阶段：
  token == u32::MAX → 无更新
  否则 → task.expect_result_by_id 取回 SparseBufferWritesSource
        → updates.write_abstract(gpu, 帧 encoder, 缓冲)
```

注意 `collector.take()` 的语义：收集器**每帧由 `use_storage_buffer` 在 Update 阶段重新创建**（[hook.rs:149](../../platform/graphics/webgpu-hook-utils/src/hook.rs#L149)），`use_update` 每帧恰好调用一次（多次调用会在第二次 panic）。

### 带宿主备份的变体：SparseUpdateStorageWithHostBuffer

[sparse_update_storage_buffer.rs:76](../../platform/graphics/webgpu-hook-utils/src/sparse_update_storage_buffer.rs#L76) 在缓冲外再包一层 `VecWithStorageBuffer`（[vec_backup.rs:3](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/vec_backup.rs#L3)），宿主侧维护一份与 GPU 同布局的 Vec：

- `use_update` 时除写 GPU 外，还把同一份稀疏更新写进宿主 Vec（`write_sparse_updates`，[sparse_update_storage_buffer.rs:120](../../platform/graphics/webgpu-hook-utils/src/sparse_update_storage_buffer.rs#L120)）。
- 宿主 Vec 通过 `buffer.make_read_holder()` 暴露为只读视图，供 host-driven（GLES）路径与 MIDC 降级路径**现场读回数据生成 DrawCommand**——例如宽线的 `params_host`（[extension/wide-line/src/indirect_draw.rs:127](../../extension/wide-line/src/indirect_draw.rs#L127)）与 attribute mesh 的 `AttributeMeshMeta` 备份（[shape/attribute/mod.rs:91](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L91)）。这就是"宿主与设备共用一套数据布局、只是写入通道不同"（material-indirect-render-guide 的分层动机之一）的实现基础。

## 范围分配器与批分配

### GrowableRangeAllocator 核心

[utility/growable-range-allocator/src/lib.rs:7](../../utility/growable-range-allocator/src/lib.rs#L7) 用 `xalloc::SysTlsf`（TLSF 算法）管理"K → 连续区域"，区域按 **item 计数**计量：

```rust
pub fn update(
  &mut self,
  change_or_removed_keys: impl Iterator<Item = K>,   // 所有"本次要动"的 key
  new: impl IntoIterator<Item = (K, Size)>,           // 这些 key 的新尺寸（item 数）
) -> BatchAllocateResult<K>
```

一次 `update` 的语义：

- 先把 `change_or_removed_keys` 里已分配的 key 全部释放（dealloc + 回收 used_count）。
- 若新需求超过剩余容量：先整体 `relocate` 到 `(used + required) * 1.1`（上限 max_item_count）的新分配器，**已有区域重排并记录数据搬迁**（`data_movements`）。
- 逐个 key 分配：失败则翻倍扩容重试；到上限仍失败记入 `failed_to_allocate`。扩容上限保护：`resize_to` 记录目标尺寸，由调用方决定何时真正 resize 缓冲。
- 空闲利用率低于一半时 `maybe_shrink`（[growable-range-allocator/src/lib.rs:191](../../utility/growable-range-allocator/src/lib.rs#L191)）缩回 `used * 2`（对齐要求 > 1 时跳过 shrink）。

调用契约（debug 断言强制）：**`new` 中出现的每个 key 都必须同时出现在 `change_or_removed_keys` 里**（即先释放再分配），否则旧区域永远不会释放、used_count 被重复计数。这也是 `use_range_allocated_device_buffers` 里 `iter_removed().chain(iter_update_or_insert())` 组合的由来。

`BatchAllocateResult`（[growable-range-allocator/src/lib.rs:267](../../utility/growable-range-allocator/src/lib.rs#L267)）的四类变更互斥：`removed`（本次删除）、`failed_to_allocate`（分配失败，**可能包含此前已分配成功过的 key**）、`data_movements`（已有数据搬迁：old_offset → new_offset）、`new_data_to_write`（新写入：offset + count）、外加 `resize_to`。`iter_update_or_insert` 把后三类合并成统一的迭代器（failed 返回 count 0，这正是"失败段的绘制命令天然无效"的约定，见 attribute-mesh-indirect-render-guide 的标记值约定）。

### BatchAllocateResultShared：GPU 落地辅助

[allocator.rs:9](../../platform/graphics/webgpu-hook-utils/src/allocator.rs#L9) 用 `Arc` 包住 `BatchAllocateResult`（廉价 clone 供多消费者），并携带 `u32_per_item`（每 item 占几个 u32，GPU 侧最小寻址单位是 4 字节）：

- `apply_resize(&mut gpu_buffer)`：若有 `resize_to`，调用 `resize_with_relocations(new_size, relocations)`——resize 的同时把 `data_movements` 转成字节级 `BufferRelocate` 一次性应用（全量拷贝 + 搬迁合一，避免二次拷贝）。**调用方负责在数据写入前完成 resize**。
- 自身实现 `DataChanges<Key = K, Value = [u32; 2]>`：`[offset, count]`（item 单位），使分配结果本身能作为"变化"流进 `update_storage_array`——这正是宽线等扩展把"段落在池里的位置"写进参数缓冲的方式。
- `DEVICE_RANGE_ALLOCATE_FAIL_MARKER = u32::MAX`（[allocator.rs:70](../../platform/graphics/webgpu-hook-utils/src/allocator.rs#L70)）：分配失败的编码值，宿主侧用 `data_range.x == u32::MAX` 跳过实体（[wide-line/src/indirect_draw.rs:486](../../extension/wide-line/src/indirect_draw.rs#L486)）。

### 写入收集：RangeAllocateBufferCollector

[allocator.rs:99](../../platform/graphics/webgpu-hook-utils/src/allocator.rs#L99) 解决"每个 key 一段数据"场景的写入打包：小数据（≤ 5KB，`SMALL_BUFFER_THRESHOLD_BYTE_COUNT`）拼进一个共享 `Vec<u8>`（记录 key → 包内偏移），大数据单独存 `Arc<Vec<u8>>`（避免大块拷贝）。`prepare`（[allocator.rs:175](../../platform/graphics/webgpu-hook-utils/src/allocator.rs#L175)）结合分配结果把小块转成 `SparseBufferWritesSource`（目标偏移 = 分配 offset × item 大小），大块保留到 render 阶段 queue 直写。产出 `RangeAllocateBufferUpdates`（[allocator.rs:211](../../platform/graphics/webgpu-hook-utils/src/allocator.rs#L211)），`write`（[allocator.rs:219](../../platform/graphics/webgpu-hook-utils/src/allocator.rs#L219)）在 render 阶段执行：先稀疏写小块（compute），再按 key 直写大块。

### 完整模板：use_range_allocated_device_buffers

[lib.rs:48](../../platform/graphics/webgpu-hook-utils/src/lib.rs#L48) 是"范围分配 + GPU 缓冲 + 数据写入"三件套的最完整示范，被宽线（[wide-line/src/indirect_draw.rs:58](../../extension/wide-line/src/indirect_draw.rs#L58)）、宽点、文字（[text-3d/src/indirect_draw.rs:55](../../extension/text-3d/src/indirect_draw.rs#L55)）、单元网格（[cell-mesh/src/indirect_draw.rs:55](../../extension/cell-mesh/src/indirect_draw.rs#L55)）、实例化模型（[transform-instanced-model/src/indirect_draw/mod.rs:61](../../extension/transform-instanced-model/src/indirect_draw/mod.rs#L61)）、attribute-mesh-lod（[attribute-mesh-lod/src/lib.rs:68](../../scene/rendering/attribute-mesh-lod/src/lib.rs#L68)）等扩展共同使用。它的流程：

```text
输入：data_source = UseResult<DataChanges<Key, Value = ExternalRefPtr<Vec<T>>>>（每个实体一份变长数据）
  ├─ use_gpu_init：按 init_item_count 创建 ResizableGPUBuffer<[T]>（item_byte_size ≥ 4 断言）
  ├─ use_sharable_plain_state：GrowableRangeAllocator::new(label, max, init, 1)
  └─ map_spawn_stage_in_thread_data_changes（worker 线程）：
       逐条变化：cast_slice 后 collect_direct 进 RangeAllocateBufferCollector；记录新尺寸
       allocator.update(removed ∪ changed, sizes) → BatchAllocateResult
       prepare 打包 → BatchAllocateResultShared
       apply_resize（此刻就 resize 缓冲，独立 encoder 提交）
       返回 Arc<RangeAllocateBufferUpdates>
  └─ fork 出两份：
       ├─ 一份 use_assure_result，CreateRender 阶段 expect_resolve_stage().write(gpu, encoder, buffer)
       └─ 一份返回给调用方（当"分配结果变化"流，喂给 update_storage_array_with_host 等）
返回 (AbstractReadonlyStorageBuffer<[T]>, UseResult<Arc<RangeAllocateBufferUpdates>>)
```

调用方拿到两份产物：GPU 缓冲直接绑定着色器；分配结果变化流写进参数缓冲（如 `WideLineParameters.data_range`），让 GPU 侧知道"这个实体的数据在池里的哪一段"。

## 其他 hook 基建

### use_db_device_foreign_key

[lib.rs:30](../../platform/graphics/webgpu-hook-utils/src/lib.rs#L30)：把外键（FK）映射成 GPU 设备侧映射表——"源实体分配索引 → 目标实体分配索引（或 u32::MAX）"：

```rust
pub fn use_db_device_foreign_key<S: ForeignKeySemantic>(
  cx: &mut QueryGPUHookCx,
) -> Option<AbstractReadonlyStorageBuffer<[u32]>> {
  // use_storage_buffer::<u32> 按 S::Entity（拥有方实体）容量增长
  // cx.use_dual_query::<S>()（反向外键查询）
  //   .map_raw_handle_or_u32_max_changes()   // Option<handle> → u32 索引或 u32::MAX
  //   .update_storage_array(cx, device_mapping_buffer, 0);
  // ...
  cx.when_render(|| device_mapping_buffer.get_gpu_buffer())
}
```

用法：`sm → std model`、`sm → node`、`sm → 宽线实体` 等一切"宿主 FK → 设备索引"映射（[gpu-indirect/src/std_model.rs:311](../../scene/rendering/gpu-indirect/src/std_model.rs#L311)、[gpu-indirect/src/node.rs:127](../../scene/rendering/gpu-indirect/src/node.rs#L127)、[wide-line/src/indirect_draw.rs:118](../../extension/wide-line/src/indirect_draw.rs#L118)）。`map_raw_handle_or_u32_max_changes` 定义在 [utility/database/src/hook/mod.rs:423](../../utility/database/src/hook/mod.rs#L423)，其姊妹 `map_some_u32_index` / `map_u32_index_or_u32_max`（同文件 [mod.rs:411](../../utility/database/src/hook/mod.rs#L411)）是材质 id 表（[std_model.rs:325](../../scene/rendering/gpu-indirect/src/std_model.rs#L325)）等场景的常用映射。

### uniform 集合与 uniform 数组

- `UniformBufferCollection<K, V>`（[use_result_ext.rs:24](../../platform/graphics/webgpu-hook-utils/src/use_result_ext.rs#L24)）：每 key 一个 `UniformBufferDataView`，`update_uniforms` 增量维护（删除移除、新增懒建后 `write_at`）。GLES/host 路径逐实体绑定的标准形态，见 [gpu-gles/src/node.rs:12](../../scene/rendering/gpu-gles/src/node.rs#L12)、[gpu-base/src/scene_id.rs:6](../../scene/rendering/gpu-base/src/scene_id.rs#L6)、[gpu-gles/src/material/unlit.rs](../../scene/rendering/gpu-gles/src/material/unlit.rs) 等。
- `UniformArray<U, N>`（[lib.rs:28](../../platform/graphics/webgpu-hook-utils/src/lib.rs#L28)）：一个定长数组 uniform（`UniformBufferDataView<Shader140Array<T, N>>`），`update_uniform_array` 按 `alloc_index * size_of::<U>() + field_offset` 写入。

### BindingArrayMaintainer

[binding_array.rs:3](../../platform/graphics/webgpu-hook-utils/src/binding_array.rs#L3)：bindless binding array 的维护器。binding array 无法增量更新，所以它每帧用一张 `SharedHashMapRead<u32, V>` 全量重建数组（超出部分填默认资源），`max_length` 限制数组长度（小则 bindless 无意义、大则重建昂贵）。gpu-base 的 bindless 纹理系统用它维护 texture / sampler 两个 binding array（[gpu-base/src/texture/mod.rs:81](../../scene/rendering/gpu-base/src/texture/mod.rs#L81)），`run_with_waked_info` 判断"本轮是否有变化"再决定是否重建。

### use_multi_access_gpu

[multi_access.rs:12](../../platform/graphics/webgpu-hook-utils/src/multi_access.rs#L12)：把 one→many 的多值引用（一个实体引用多个实体，如"一盏灯属于多个场景"）投影成 GPU 侧可迭代结构，输入是 `TriQueryLike`（many→one 反向视图 + 变化）：

- **many 侧**：一个 `[u32]` 索引池，按"每个 one 一段连续区域"由 `GrowableRangeAllocator` 分配（复用 `RangeAllocateBufferCollector` 打包写入）。
- **one 侧**：一个 `[GPURangeInfo { start, len }]` 元数据表（按 one 的分配索引），`GPURangeInfo` 默认值 `start = u32::MAX` 表示"已删除/空"（[multi_access.rs:194](../../platform/graphics/webgpu-hook-utils/src/multi_access.rs#L194)）。
- 渲染侧消费：`MultiAccessGPUInvocation`（[multi_access.rs:210](../../platform/graphics/webgpu-hook-utils/src/multi_access.rs#L210)）提供 `iter_refed_many_of(one)`（shader 迭代器，按 meta 的范围遍历索引池）与 `get_n_th(one, n)`（直接取第 n 个），供点光/聚光/方向光按场景枚举灯光（[gpu-indirect/src/light/spot.rs:53](../../scene/rendering/gpu-indirect/src/light/spot.rs#L53)）、裁剪平面数组（[effect/plane_array_clip/src/lib.rs:75](../../effect/plane_array_clip/src/lib.rs#L75)）等场景使用。

## 用户视角：下游用法一览

### 标准套路

一个"实体槽位数据"的典型装配（以 [gpu-base/src/world_matrix.rs:15](../../scene/rendering/gpu-base/src/world_matrix.rs#L15)、[material/mr.rs:6](../../scene/rendering/gpu-indirect/src/material/mr.rs#L6)、[light/spot.rs:15](../../scene/rendering/gpu-indirect/src/light/spot.rs#L15) 归纳）：

```rust
let (cx, storage) = cx.use_storage_buffer(label, init_capacity, u32::MAX); // 1. 建缓冲
// 2. 每路组件变化 → 稀疏写（字段级偏移），一个实体多字段就多路 update_storage_array
cx.use_changes::<SomeComponent>().map_changes(transform).update_storage_array(cx, storage, offset_of!(Storage, field));
// 3. 容量同步
storage.use_max_item_count_by_db_entity::<SomeEntity>(cx);
// 4. 两阶段收口
storage.use_update(cx);
// 5. render 阶段输出 GPU 句柄
cx.when_render(|| storage.get_gpu_buffer())
```

### 下游分布

| 下游 | 用法 | 位置 |
| --- | --- | --- |
| gpu-base（世界矩阵/包围盒/相机/scene id） | `use_storage_buffer` + `update_storage_array`；`use_uniform_buffers` + `update_uniforms` | [world_matrix.rs:15](../../scene/rendering/gpu-base/src/world_matrix.rs#L15)、[scene_id.rs:6](../../scene/rendering/gpu-base/src/scene_id.rs#L6) |
| gpu-indirect 材质 | 每类材质参数/句柄双存储缓冲 + 字段级稀疏写 | [material/mr.rs:6](../../scene/rendering/gpu-indirect/src/material/mr.rs#L6) |
| gpu-indirect 网格 | 元数据表带宿主备份；顶点/索引池用范围分配器 | [shape/attribute/mod.rs:91](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L91) |
| gpu-indirect 灯光 | `use_storage_buffer` + `use_multi_access_gpu` | [light/spot.rs:15](../../scene/rendering/gpu-indirect/src/light/spot.rs#L15) |
| gpu-gles（host 路径） | `use_uniform_buffers` 逐实体 uniform | [gpu-gles/src/node.rs:12](../../scene/rendering/gpu-gles/src/node.rs#L12) |
| batch-extractor | `SparseBufferWritesSource` + `GrowableRangeAllocator` 维护 id 池 | [list_pool.rs:70](../../scene/rendering/batch-extractor/src/list_pool.rs#L70) |
| extension（宽线/宽点/文字/单元网格/实例化/LOD） | `use_range_allocated_device_buffers` + `use_storage_buffer_with_host_backup` + `use_db_device_foreign_key` | [wide-line/src/indirect_draw.rs:37](../../extension/wide-line/src/indirect_draw.rs#L37) |
| effect（csg clip / plane array clip） | `use_uniforms` 与 `use_multi_access_gpu` | [effect/plane_array_clip/src/lib.rs:75](../../effect/plane_array_clip/src/lib.rs#L75) |

## 使用规则

1. **GPU 资源必须用 `use_gpu_init` / `use_state_with_features` 创建**，保证跨帧单例；渲染器对象只应在 `when_render` 内组装。
2. **`update_storage_array` 必须在 spawn 阶段调用**（写入任务经由收集器收集）；不要对变化链先 `use_assure_result` 再 update（resolve 阶段会因"未在 spawn 阶段预备"而在 debug 构建 panic）。
3. **`use_update` 每帧恰好调用一次**（收集器被 take，第二次调用 panic）；`use_storage_buffer` 每帧在 Update 阶段重建收集器。
4. **稀疏写目标区间不得重叠**（同实体同字段多路写入时用字段偏移错开，同实体不同字段是不同区间，天然安全）；数据与偏移都需 4 字节对齐（item_byte_size ≥ 4 有断言）。
5. **范围分配器的 key 生命周期**：`new` 里出现的 key 必须同时在 `change_or_removed_keys` 里（先释放再分配）；`BatchAllocateResultShared::apply_resize` 必须先于数据写入执行。
6. **缓冲容量**：槽位下标是实体分配索引，索引超出缓冲容量是越界访问——`use_max_item_count_by_db_entity`（或 resize）必须在写入前保证容量；`extra-checks` feature（rangemap）会检测写越界与重叠。
7. **跨帧 token 语义**：`use_plain_state` 里的 token（u32::MAX 表示"无"）在 Update 阶段写入、CreateRender 阶段读取，两个阶段必须都执行同一调用点（hook 形状稳定是前提）。
8. **分配失败用 `DEVICE_RANGE_ALLOCATE_FAIL_MARKER` 表达**：宿主侧检查后跳过实体，GPU 侧保证失败段 count 为 0（空绘制命令无效果）。

## 常见疑问

- **为什么稀疏写用 compute 派发而不是 queue.write_buffer**：一次派发完成所有碎片写、与帧编码同序；queue 直写只在"目标不是真 buffer（texture 模拟）"时作为退化路径。
- **为什么数据写入分 spawn / render 两阶段**：打包/合并/分配可在 worker 线程并行，GPU 写入必须落在帧 encoder 上与渲染 pass 顺序一致；`SparseBufferWritesSource` 就是两个阶段之间的"可传输"载体（任务结果）。
- **什么时候用 `SparseUpdateStorageWithHostBuffer`**：host 侧需要在渲染时读回数据（生成 DrawCommand、做降级决策）的场景——宽线/宽点/文字的参数、attribute mesh 元数据、LOD 元数据。纯 GPU 消费的数据用普通版本即可。
- **`u32::MAX` 为什么无处不在**：实体分配索引 0 是合法槽位，所以"无/失败/空"统一用 u32::MAX 编码，着色器侧按哨兵分支（详见 material-indirect-render-guide 的句柄语义）。

## 延伸阅读

- 本模块被引用的具体机制在各自文档中的展开：材质参数槽与句柄（[material-indirect-render-guide.md](material-indirect-render-guide.md)）、id 池与批分组（[batch-extractor-guide.md](batch-extractor-guide.md)）、网格元数据与 MIDC 降级（[attribute-mesh-indirect-render-guide.md](attribute-mesh-indirect-render-guide.md)）
- 缓冲资源层组合子：[platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/mod.rs](../../platform/graphics/webgpu/src/resource/buffer/linear_buffer_array/mod.rs)、[abstract_resource.rs](../../platform/graphics/webgpu/src/resource/buffer/abstract_resource.rs)
- 范围分配器本体：[utility/growable-range-allocator/src/lib.rs](../../utility/growable-range-allocator/src/lib.rs)
- 两阶段与 UseResult 的通用机制：[query-hook-guide.md](query-hook-guide.md)、[utility/query-hook/src/use_result.rs](../../utility/query-hook/src/use_result.rs)
- 帧时序与渲染器装配：[application/viewer-content/src/rendering_root.rs](../../application/viewer-content/src/rendering_root.rs)、[application/viewer-content/src/rendering/frame_all.rs](../../application/viewer-content/src/rendering/frame_all.rs)
