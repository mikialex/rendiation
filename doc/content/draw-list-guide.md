# GPU Draw-List 模式理解（shader/draw-list）

本文档是对 [shader/draw-list](../../shader/draw-list/src/lib.rs) 实现的模式梳理，覆盖 GPU 驱动绘制列表（DeviceDrawList）、多范围绘制描述（MultiRange）、GPU 剔除抽象（AbstractCullerProvider）与 GPU 流压缩（stream_compact），以及它们在下游（batch-extractor、frustum/occlusion culling、gpu-indirect）如何被消费。

## 前置阅读

- [shader-edsl-core-zh.md](skill-translation/shader-edsl-core-zh.md)：`Node<T>`、shader struct、控制流等阶段无关语言原语
- [shader-edsl-compute-zh.md](skill-translation/shader-edsl-compute-zh.md)：计算管线构建、工作组内存、内置计算 ID
- [shader-edsl-binding-and-typed-container-zh.md](skill-translation/shader-edsl-binding-and-typed-container-zh.md)：`StorageBufferDataView` 等类型化资源容器与双向绑定
- [fundamental-gpu-component-model-zh.md](skill-translation/fundamental-gpu-component-model-zh.md)：`ShaderHashProvider`、`RenderComponent` 等可组合 GPU 组件模型
- [frame-pass-assemble-zh.md](skill-translation/frame-pass-assemble-zh.md)：渲染帧的组装方式
- [scene-core-structure-zh.md](skill-translation/scene-core-structure-zh.md)：场景模型（SceneModelEntity）等实体模型

## 为什么需要 GPU draw list

CPU 驱动的渲染每帧要做三件事：遍历场景模型、做剔除、逐个提交绘制命令。场景模型数量大、每帧重复时，CPU 侧遍历与剔除会成为瓶颈；而剔除（视锥、遮挡）恰好是天然适合 GPU 的并行工作。

rendiation 的解法是"把绘制列表本身搬上 GPU"：

- 场景模型的 id 列表存进 GPU buffer（称为 id pool）。
- 一组模型可能按材质/网格分成若干"子列表"（sub-list），每个子列表对应一类绘制实现。用一个范围描述符数组（offset / count / 前缀和）描述这些子列表在 pool 中的位置。
- 剔除用 compute shader 对 id 逐个求"是否存活"，再用流压缩（prefix scan + scatter）把存活 id 原地压紧到各子列表区域前端，同时把新 count 写回范围描述符。
- 渲染端直接以范围描述符的 count 字段作为间接绘制（MultiDrawIndirectCount）的每段数量，GPU 自己决定画多少。

这样剔除与提交完全在 GPU 上闭环：CPU 只需要准备初始 id 列表与范围描述，剔除结果自动流向下一次间接绘制。剔除链路（frustum、occlusion）各环节都只消费同一份 `DeviceDrawList`，这正是本 crate 的核心价值。

## 抽象体系总览

核心数据结构与 trait 一览：

| 概念 | 定义位置 | 作用 |
| --- | --- | --- |
| `DeviceDrawList` | [shader/draw-list/src/lib.rs](../../shader/draw-list/src/lib.rs) | GPU 绘制列表：id pool + 派发信息 |
| `MultiRangeDispatchInfo` | 同上 | 多范围派发信息（设备侧 + 主机侧 + 总容量） |
| `StorageSubListRangeInfo` | [shader/draw-list/src/multi_range.rs](../../shader/draw-list/src/multi_range.rs) | 单个子列表的 GPU 范围描述（offset/count/前缀和） |
| `DeviceMultiRangeDispatchInfo` | 同上 | 子列表范围数组 + 总存活数标量 |
| `AbstractCullerProvider` | [shader/draw-list/src/device_culling/mod.rs](../../shader/draw-list/src/device_culling/mod.rs) | 剔除 provider 抽象（宿主侧） |
| `AbstractCullerInvocation` | 同上 | 剔除 invocation 抽象（着色器侧） |
| `SceneModelCullingComponent` | [shader/draw-list/src/device_culling/culling.rs](../../shader/draw-list/src/device_culling/culling.rs) | 把 id 流与 culler 组合成 `ComputeComponent` |
| `ListOfListsCullingPredicate` | [shader/draw-list/src/stream_compact/predicate.rs](../../shader/draw-list/src/stream_compact/predicate.rs) | 剔除 mask（0/1 流）生成 |
| `SegmentedListScatter` | [shader/draw-list/src/stream_compact/scatter.rs](../../shader/draw-list/src/stream_compact/scatter.rs) | 按前缀和 scatter 存活 id 并回写范围信息 |
| `ComputeComponent<T>` / `DeviceInvocation<T>` | [shader/parallel-compute/src/abstract_component.rs](../../shader/parallel-compute/src/abstract_component.rs)、[abstract_invocation.rs](../../shader/parallel-compute/src/abstract_invocation.rs) | GPU 并行计算的组件抽象（上游） |

trait 体系的层次关系：

- `ComputeComponent<T>`（来自 parallel-compute）：一个可派发的 GPU 计算单元。宿主侧回答"工作量多大"（`work_size`）、"结果多大"（`result_size`）、"工作组多大"（`requested_workgroup_size`）；`build_shader` 在管线构建时返回着色器侧对应物 `DeviceInvocation<T>`；`bind_input` 负责把资源绑进派发 pass。
- `DeviceInvocation<T>`：着色器侧的"单线程执行体"。`invocation_logic(global_id)` 返回 `(结果, valid)`，`invocation_size()` 返回总线程数（GPU 端计算）。
- `AbstractCullerProvider`：宿主侧的剔除器描述，自带 shader 哈希（管线缓存键）；`AbstractCullerInvocation` 是它在着色器侧的对应物，核心方法是 `cull(id: Node<u32>) -> Node<bool>`，返回 true 表示该 id 应被剔除（不可见）。

`DeviceDrawList` 本身实现了 `ComputeComponent<Node<Vec2<u32>>>`（见 [list_access.rs](../../shader/draw-list/src/list_access.rs)），因此它可以被当作任何并行计算的输入：每个线程输出 `(scene_model_id, sub_list_index)`。

## 核心数据结构

### DeviceDrawList

```rust
pub struct DeviceDrawList {
  pub id_pool: StorageBufferReadonlyDataView<[u32]>,
  pub dispatch_info: MultiRangeDispatchInfo,
}
```

- `id_pool`：所有子列表的模型 id 拼接成的连续 buffer（按子列表容量排布，各子列表区域从自己的 offset 开始）。
- `dispatch_info`：派发信息，见下。

### MultiRangeDispatchInfo 与 CapacityRange

```rust
pub struct MultiRangeDispatchInfo {
  pub device_ranges: DeviceMultiRangeDispatchInfo,
  pub host_capacity_ranges: Vec<CapacityRange>,
  pub total_capacity: u32,
}

pub struct CapacityRange {
  /// must not equal to zero
  pub capacity: u32,
  pub offset: u32,
}
```

- `host_capacity_ranges` 是主机侧的子列表布局：每个子列表的**容量**（预分配的上限）与在 pool 中的起始偏移。capacity 必须非零。
- `total_capacity` 是所有 capacity 之和，即 id pool 的元素个数。
- `device_ranges` 是 GPU 侧的实际范围描述（见下）。

### StorageSubListRangeInfo

```rust
#[repr(C)]
#[std430_layout]
pub struct StorageSubListRangeInfo {
  pub offset: u32,           // pool_read_range_offset：该子列表在 id pool 中的读取偏移
  pub count: u32,            // 元素数量（剔除后会由 GPU 回写为存活数）
  pub count_prefix_sum: u32, // 排他前缀和（该子列表之前所有 count 之和）
}
```

三个字段的语义分别在两个用途中体现：

- **读取**：给定全局线程 id g（在"总 count"空间内），先二分查找 count_prefix_sum <= g 的最后一个子列表 i，则 `id = id_pool[g - count_prefix_sum[i] + offset[i]]`，这就是 `DeviceMultiRangeDispatchInfoInvocation::compute_list_index`（[multi_range.rs](../../shader/draw-list/src/multi_range.rs)）做的事。
- **间接绘制**：`create_indirect_count_views`（[lib.rs](../../shader/draw-list/src/lib.rs)）对每个子列表在 `sub_list_ranges` buffer 上创建 `offset = i * size + 4`、`size = 4` 的 buffer view——正好落在 count 字段上。这个 view 被当作 MultiDrawIndirectCount 的每段绘制数量。剔除完成后 count 字段被 shader 更新为存活数，绘制数量自动精确。

`count_prefix_sum` 与 `offset` 的区别：prefix sum 是对"逻辑元素"计数（剔除前的输入元素数），offset 是对"物理容量"计数（pool 中的位置）。两者在初始时通常数值相近，但 capacity 做了对齐/预留后就会出现差异。

### 主机侧辅助函数

`prepare_gpu_sub_list_ranges(host_capacity_ranges, real_length)`（[lib.rs](../../shader/draw-list/src/lib.rs)）把主机侧容量范围与当前真实长度合并成 GPU 初始化用的 `StorageSubListRangeInfo` 数组：offset 用 capacity 的 offset，count 用 real_length，前缀和逐段累加。剔除前的初始化一般传入全零 real_length，由 GPU 在剔除时写入真实 count。

`DeviceMultiRangeDispatchInfo::new` / `update` 负责创建 / 更新这两个 GPU buffer。注意 `sub_list_ranges` 创建时带 `BufferUsages::INDIRECT` 标志，因为它的 count 字段会被间接绘制当作计数 buffer 使用。

### 剔除输出目标的创建与缓存

`DeviceDrawList::create_or_update_compact_culling_write_target`（[lib.rs](../../shader/draw-list/src/lib.rs)）为"剔除后的 compact 输出"创建（或复用）目标：

- 按 `host_capacity_ranges` 的总容量建一个新的全零 id pool（`id_pool`）。
- 建新的 `DeviceMultiRangeDispatchInfo`（count 全零，等待 GPU 写入）。
- 如果总容量或子列表数量与缓存一致，就只更新初始化范围而不重建 buffer。

这是 hook 风格的状态缓存：`use_culled_list_and_do_culling` 里用 `cx.use_plain_state_default::<Option<DeviceDrawList>>()` 持有上一帧的剔除输出，避免每帧重新分配 GPU buffer。

## list_access：shader 端读取 draw list

[list_access.rs](../../shader/draw-list/src/list_access.rs) 让 `DeviceDrawList` 成为平行计算的输入源：

- 实现 `ComputeComponent<Node<Vec2<u32>>>`，`result_size` 是总容量，`work_size` 为 None（使用间接派发，见下文 invocation_size）。
- `invocation_logic`：`global_id = logic_global_id.x()`，先用 `compute_list_index` 得到 `(list_index, in_bound)`，再用范围信息把全局 id 映射到 pool 中的物理位置，读出模型 id，返回 `(vec2(id, list_index), in_bound)`。
- `invocation_size`：`sum_all_count`（所有子列表 count 之和，GPU 端标量），因此**派发规模由 GPU 端数据决定**（间接派发）。剔除后 sum_all_count 被回写为存活总数，后续所有消费它的派发规模自动跟随。

这也是"多范围派发"（multi-range dispatch）的核心：一个 compute 派发覆盖 N 个不连续的子列表，线程在逻辑上从 0 连续编号到总 count，通过二分查找路由到正确的子列表。

## device_culling：GPU 剔除抽象

### trait 定义

```rust
pub trait AbstractCullerProvider: ShaderHashProvider + DynClone {
  fn create_invocation(&self, cx: &mut ShaderBindGroupBuilder) -> Box<dyn AbstractCullerInvocation>;
  fn bind(&self, cx: &mut BindingBuilder);
}

pub trait AbstractCullerInvocation {
  /// return if the item should be culled, true == invisible
  fn cull(&self, id: Node<u32>) -> Node<bool>;
}
```

`AbstractCullerProvider` 是宿主侧对象（可 clone、可参与管线哈希、负责把资源 bind 进 pass），`AbstractCullerInvocation` 是着色器侧执行体（持有 shader 指针，逐 id 判定）。因为管线按 shader 哈希缓存，culler 的类型与绑定的资源都参与 `ShaderHashProvider` 的哈希，不同剔除逻辑自动获得不同的管线。

`Box<dyn AbstractCullerProvider>` 也实现了该 trait（转发），所以剔除器可以自由装箱嵌套。

### 组合子

`AbstractCullerProviderExt`（[mod.rs](../../shader/draw-list/src/device_culling/mod.rs)）为可 clone 的 provider 提供组合：

- `not()`：取反，`NotCuller`。
- `shortcut_or(other)`：短路或，`ShortcutOrCuller`——先算左边的 cull 结果，为 false（可见）时才继续算右边（见 [operator.rs](../../shader/draw-list/src/device_culling/operator.rs)）。用于"预剔除（如 frustum）＋昂贵的遮挡测试"组合，遮挡测试只在 frustum 未剔除时才执行。

[operator.rs](../../shader/draw-list/src/device_culling/operator.rs) 还提供 `NoopCuller`（什么都不剔除），作为测试与缺省实现。

### SceneModelCullingComponent

[device_culling/culling.rs](../../shader/draw-list/src/device_culling/culling.rs) 把"输入 id 流 + 一个 culler"组合成完整的 `ComputeComponent<Node<bool>>`：输入可以是任何输出 u32 的组件（典型就是 `DeviceDrawList`，输出 model id），`invocation_logic` 里在 valid 时求 `cull(id).not()` 作为保留标志。这是 predicate（mask）的标准入口。

### 下游 culler 实现

| 实现 | 位置 | 逻辑 |
| --- | --- | --- |
| `GPUFrustumCuller` | [scene/rendering/frustum-culling/src/lib.rs](../../scene/rendering/frustum-culling/src/lib.rs) | 用 6 平面视锥 + AABB 相交测试，相机世界位置与视锥数据来自 UBO |
| `OnlyLastFrameVisible` | [scene/rendering/occlusion-culling/src/filter.rs](../../scene/rendering/occlusion-culling/src/filter.rs) | 按 `last_frame_invisible` 可见性 buffer 判定（上一帧被遮挡视为候选剔除） |
| `OcclusionTester` | [scene/rendering/occlusion-culling/src/occlusion_test.rs](../../scene/rendering/occlusion-culling/src/occlusion_test.rs) | AABB 8 角点投影到上一帧深度金字塔采样做遮挡判定，并把结果写回可见性 buffer（测试即更新） |

`OcclusionTester` 值得注意：它的 `cull` 有副作用（写 `last_frame_invisible`），因此被单独用一个手工管线跑一遍"只测试上一帧可见物体"，再作为 culler 用于第二遍剔除。这也解释了 cull 方法的语义约定是"返回是否剔除"，实现者可以在其中做任意 shader 逻辑。

## stream_compact：GPU 流压缩

流压缩（stream compaction）的目标：给定每个元素的保留标志，把保留元素压到数组前端，得到新长度。这里还要求**按子列表分段压缩**，并回写每段的新 count 与前缀和。

### 算法总览

`DeviceDrawList::use_culled_list_and_do_culling`（[stream_compact/mod.rs](../../shader/draw-list/src/stream_compact/mod.rs)）是入口，三个阶段：

- 求 mask（predicate）：`ListOfListsCullingPredicate` 对每个 (id, sub_list) 求 1/0。
- 分段前缀和：mask 经 `use_segmented_prefix_scan_kogge_stone::<AdditionMonoid<u32>>` 得到全局 inclusive 前缀和数组 positions（每个元素的"在全局存活序列中的位置"）。
- 分段 scatter：`SegmentedListScatter` 按 positions 把存活 id 写入输出 pool 的紧凑区域，同时由前 K 个线程从边界值推导各子列表新 count 与前缀和，写入输出 ranges 与总 count。

### predicate：mask 生成

`ListOfListsCullingPredicate`（[predicate.rs](../../shader/draw-list/src/stream_compact/predicate.rs)）实现 `ComputeComponent<Node<u32>>`，内部就是 `DeviceDrawList`（读 id）套 `culler`（判存活），输出 1/0。它的 `work_size` 是 None（走间接派发，规模由 `sum_all_count` 决定），`result_size` 是总容量。

### 与 parallel-compute 的关系

前缀和来自 [shader/parallel-compute](../../shader/parallel-compute/src/lib.rs) 的 `use_segmented_prefix_scan_kogge_stone`——Kogge-Stone 工作组内扫描（共享内存 + 双屏障）与两阶段全局扫描的完整算法、monoid 抽象（`DeviceMonoidLogic` / `AdditionMonoid<u32>`）见 [parallel-compute-primitives-guide.md](parallel-compute-primitives-guide.md) 的「工作组内扫描：Kogge-Stone」与「两阶段全局扫描：段前缀和」。

draw-list 传给它的 workgroup size 是 `max_compute_invocations_per_workgroup`（设备上限），即单 stage 尽量大、两次扫描覆盖全量。

通用的流压缩原语 `use_stream_compaction` 也在 [parallel-compute/src/stream_compaction.rs](../../shader/parallel-compute/src/stream_compaction.rs)，思路相同（mask → scan → shuffle_move）。draw-list 的 `SegmentedListScatter` 是其"分段 + 元数据回写"的特化版本，直接手工 scatter 以省一次 shuffle。

### scatter：写入与元数据回写

`SegmentedListScatter`（[scatter.rs](../../shader/draw-list/src/stream_compact/scatter.rs)）单个派发内做两件事：

- 线程 0..K-1（K = 子列表数）更新元数据：对子列表 i，`p_prev` = 前一个子列表末尾的 inclusive 前缀和（首段为 0），`p_end` = 本段末尾的 inclusive 前缀和，则新 `count = p_end - p_prev`、新排他前缀和 = `p_prev`，写入 `output_ranges[i]`；最后一个子列表线程把 `p_end` 写入 `total_count_out`（总存活数）。
- 所有线程 scatter：`keep[i] ⇔ p[i] != p[i-1]`（首元素为 `p[0] > 0`），从 mask 恢复保留标志而不需要额外 mask buffer。`seg_start` = 本子列表之前所有存活数（前一个子列表末尾的 p），`local_pos = p_i - seg_start - 1`，把 model id 写入 `output_pool[seg_start + local_pos]`。

**空子列表保护**是这个 shader 的经典坑：当某个（或多个连续的）子列表全部被剔除后，`count_prefix_sum + count == 0`，直接读 `positions[count_prefix_sum + count - 1]` 会 u32 下溢。scatter 用 `select_branched` 对 `prev_is_empty_prefix` / `is_empty_prefix` 做保护，逐段回退到更早的边界值（[tests.rs](../../shader/draw-list/src/stream_compact/tests.rs) 中覆盖了空首段、连续空段、中间空段等全部组合，是理解该 shader 行为的最佳材料）。

### 输出复用

输出 pool 与 ranges 由 `create_or_update_compact_culling_write_target` 缓存，`use_culled_list_and_do_culling` 每次把 `scatter.output_pool` 转成 readonly view，拼出新的 `DeviceDrawList` 返回。宿主侧 `host_capacity_ranges` 与 `total_capacity` 不变（容量不因剔除收缩），`device_ranges` 指向新写的 ranges。

## 下游：谁生产、谁剔除、谁提交

### 生产：batch-extractor 与间接场景渲染器

两条生产路径：

- [scene/rendering/batch-extractor/src/extractor.rs](../../scene/rendering/batch-extractor/src/extractor.rs) 的 `IncrementalDeviceSceneBatchExtractor`：增量维护 `(scene, group_key)` → 模型 id 列表。`SceneModelListPool`（[list_pool.rs](../../scene/rendering/batch-extractor/src/list_pool.rs)）用 `GrowableRangeAllocator` 给每个组在 pool 中分配容量区域。`extract_scene_batch` 读各组的 capacity/offset 组装 `DeviceDrawList`，与 `impl_select_ids`（每组一个代表模型，用于选择绘制实现）一起包成 `DeviceSceneModelDrawList`（[scene/rendering/gpu-base/src/batch.rs](../../scene/rendering/gpu-base/src/batch.rs)）。
- [scene/rendering/gpu-indirect/src/scene.rs](../../scene/rendering/gpu-indirect/src/scene.rs) 的 `IndirectSceneRenderer::create_batch_from_iter`：对任意模型迭代器按 `hash_shader_group_key` 分类，直接建 pool（容量按 storage buffer offset 对齐取整），作为 Host batch 到 Device batch 的转换路径。

### 剔除：viewer 的接线

[application/viewer-content/src/rendering/culling.rs](../../application/viewer-content/src/rendering/culling.rs) 是用户视角的完整链路：

- `use_execute_frustum_culler`：把 `GPUFrustumCuller` 通过 `use_culled_list_and_do_culling` 施加到 batch 上（仅在遮挡剔除关闭时启用，两者都做视锥判断）。
- `use_draw_with_oc_maybe_enabled`：进入 [scene/rendering/occlusion-culling/src/lib.rs](../../scene/rendering/occlusion-culling/src/lib.rs) 的 `GPUTwoPassOcclusionCulling::use_draw`——先用"上一帧可见性"把 batch 拆成两半（`filter_last_frame_visible_object(...).not()` 就是 culler 组合的典型应用），第一遍画上一帧可见者作为 occluder，生成深度金字塔，`OcclusionTester` 更新可见性并得到遮挡 culler，第二遍用 `pre_culler.shortcut_or(occlusion_culler)` 剔除剩余部分后绘制。完整机制见 [occlusion-culling-guide.md](occlusion-culling-guide.md)。

### 提交：gpu-indirect 的间接绘制

[scene/rendering/gpu-indirect/src/scene.rs](../../scene/rendering/gpu-indirect/src/scene.rs) 的 `use_make_scene_batch_pass_content`：按实现二次分类子列表 → `use_compute_selected_sub_list_dispatch_info` 重算两套 `MultiRangeDispatchInfo`（原始池偏移 / 紧凑偏移）→ `use_create_or_update_indirect_draw_providers` 经 mid 层生成间接命令——这条组织链路的完整实现见 [gpu-indirect-batch-collector-guide.md](gpu-indirect-batch-collector-guide.md) 的「use_make_scene_batch_pass_content 的实现」，这里只强调与 draw-list 的衔接点：

- 命令生成在 [scene/rendering/gpu-base/src/mid/mod.rs](../../scene/rendering/gpu-base/src/mid/mod.rs) 的 `use_and_create_default_indirect_draw_provider` 完成：`IndexedDrawCommandGeneratorComponent`（[indexed.rs](../../scene/rendering/gpu-base/src/mid/indexed.rs)）按每个 model id 生成 `DrawIndexedIndirectArgsStorage` 命令写入命令池（按子列表 capacity 排布）。
- 绘制数量直接复用本节的 count 语义：`create_indirect_count_views`（[multi_range.rs:39](../../shader/draw-list/src/multi_range.rs#L39)）把每段 ranges 的 count 字段切成 buffer view，作为 `MultiIndirectDrawBatch` 的 `MultiIndirectCount` 每段绘制数量——剔除回写 count，绘制端无需 CPU 干预。

不支持 `MULTI_DRAW_INDIRECT_COUNT` 的平台（DX12 已知问题、功能缺失，见 [platform/graphics/webgpu-midc-downgrade/src/lib.rs](../../platform/graphics/webgpu-midc-downgrade/src/lib.rs) 的 `require_midc_downgrade`）走 `use_downgrade_multi_indirect_draw_count_list_pool`：对每段再做一次前缀和，拆成单段 `DrawIndirect`，由 `MIDCDowngradeBatch` 在顶点阶段注入段内基址与段基址（`VertexIndexForMIDCDowngradeBaseIndex` 等），保证顶点索引语义一致。完整降级机制见 [indirect-draw-command-guide.md](indirect-draw-command-guide.md) 的「MIDC 降级机制」。

## 使用规则

- **capacity 必须非零**，offset 必须按 storage buffer offset alignment 对齐（id pool 的元素单位是 u32，对齐值除以 4）。
- **culler 的语义**：`cull(id)` 返回 true == 剔除（不可见）。想要"保留"请在 culler 外层 `not()`，或在 `SceneModelCullingComponent` 的消费中取反。
- **剔除输出是可重入的**：`use_culled_list_and_do_culling` 的输入可以是上一次剔除的输出（例如遮挡剔除里对"上一帧可见子集"再套 pre_culler），输出 buffer 由 hook 状态缓存复用，capacity 不变。
- **不要手改 count 字段**：count 与 sum_all_count 由 scatter shader 回写，宿主只负责容量。初始化时 count 全零，此时派发规模为 0（间接派发会正确跳过）。
- **shader 哈希**：任何自定义 culler 必须实现 `ShaderHashProvider`（通常只写 `shader_hash_type_id! {}`），culler 类型 id 与绑定资源的类型都会进入管线哈希，不同剔除逻辑与绑定布局自动获得不同管线；若 shader 逻辑依赖宿主侧的布尔/常量，需要显式把它们 hash 进去。
- **分支注意**：在 `use_culled_list_and_do_culling` 这类 hook 调用外不要引入随帧变化的条件创建新状态（见 [hooks-guide.md](hooks-guide.md) 的规则）。
- 测试验证（[stream_compact/tests.rs](../../shader/draw-list/src/stream_compact/tests.rs)）展示了纯 GPU 单元的验证方式：`DeviceParallelComputeCtx` + `read_storage_u32` 读回 buffer 断言，新增 culler 逻辑时可仿照。

## 使用模板：自定义剔除器

以"按模型 id 与 UBO 中的阈值剔除"为例，展示一个完整的最小实现：

```rust
use rendiation_device_draw_list::*;
use rendiation_device_parallel_compute::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;
use rendiation_webgpu_hook_utils::*;

#[derive(Clone)]
struct IdThresholdCuller {
  threshold: UniformBufferDataView<u32>,
}

impl ShaderHashProvider for IdThresholdCuller {
  shader_hash_type_id! {}
}

impl AbstractCullerProvider for IdThresholdCuller {
  fn create_invocation(
    &self,
    cx: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn AbstractCullerInvocation> {
    Box::new(IdThresholdCullerInvocation {
      threshold: cx.bind_by(&self.threshold),
    })
  }

  fn bind(&self, cx: &mut BindingBuilder) {
    cx.bind(&self.threshold);
  }
}

struct IdThresholdCullerInvocation {
  threshold: ShaderReadonlyPtrOf<u32>,
}

impl AbstractCullerInvocation for IdThresholdCullerInvocation {
  fn cull(&self, id: Node<u32>) -> Node<bool> {
    id.greater_than(self.threshold.load())
  }
}
```

应用：假设 `draw_list: DeviceDrawList` 已在某处构造好（例如从 batch extractor 的 `DeviceSceneModelDrawList.draw_list` 拿），在渲染帧内剔除并提交：

```rust
// 帧内，cx 是 DeviceParallelComputeCtx（经 FrameCtx::access_parallel_compute 获得）
let culled = draw_list.use_culled_list_and_do_culling(cx, Box::new(culler));

// culled 的 id_pool 前部是存活 id，ranges 的 count 是各子列表存活数
// 交给 indirect draw provider 或用于其它消费：
let _ = culled.create_indirect_count_views(); // 每段的间接绘制数量 view
```

组合剔除器的典型写法：

```rust
let culler = IdThresholdCuller { threshold }.not();                       // 取反
let culler = NoopCuller.not();                                            // 等价于"全部保留"的 culler
let culler = cheap_culler.shortcut_or(expensive_culler);                  // 短路或
```

## 常见坑速查

- 空子列表的 `count_prefix_sum + count - 1` 下溢：已由 scatter 保护，但自己写的并行扫描逻辑也要注意同类边界。
- `capacity` 为 0：`create_or_update_compact_culling_write_target` 依赖 capacity 非零，空组应在生产侧过滤（batch-extractor 在 `new_size == 0` 时删除组）。
- 忘记 INDIRECT usage：`sub_list_ranges` 与命令池 buffer 必须带 `BufferUsages::INDIRECT`，否则 `create_indirect_count_views` / 间接绘制在 wgpu 校验层报错。
- hash 的是"影响生成 shader 代码"的宿主数据（如 `OcclusionTester` 的 `reverse_depth` 布尔、捕获的 shader 常量），不是运行时绑定数据：UBO 内容（如相机矩阵、视锥平面）在 bind 时变化，不参与管线哈希，否则管线缓存会失效。
