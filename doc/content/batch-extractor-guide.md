# Rendiation 场景批提取与增量 PSO Key 指南（scene/rendering/batch-extractor）

本文梳理 [scene/rendering/batch-extractor](../../scene/rendering/batch-extractor/src/lib.rs) 的场景批提取（batch extraction）体系：增量 PSO key（render pipeline / pass 相关 key）如何维护与计算、实体 id 如何在 GPU 池中按 key 分组常驻，以及 [extension/occ-style-draw-control](../../extension/occ-style-draw-control/src/lib.rs) 扩展如何在这套机制上叠加分层绘制控制。

## 前置阅读

批提取建立在增量查询、两阶段 hook 执行与 GPU 绘制列表之上，建议先了解：

| 文档 | 内容 |
| --- | --- |
| [skill-translation/query-system-zh.md](skill-translation/query-system-zh.md) | DualQuery 增量模型、组合子（select/intersect/union/filter_by_set）与 fanout 扇出 |
| [query-hook-guide.md](query-hook-guide.md) | 两阶段执行模型（spawn/resolve）、UseResult 四态、共享计算 |
| [skill-translation/database-schema-zh.md](skill-translation/database-schema-zh.md) | 组件、外键、反向引用视图（rev ref tri view） |
| [skill-translation/scene-core-structure-zh.md](skill-translation/scene-core-structure-zh.md) | SceneModelEntity / StandardModelEntity、payload 外键、节点可见性 |
| [skill-translation/fundamental-gpu-component-model-zh.md](skill-translation/fundamental-gpu-component-model-zh.md) | ShaderHashProvider / ShaderPassBuilder 与管线哈希 |
| [draw-list-guide.md](draw-list-guide.md) | DeviceDrawList、多范围绘制（MultiRangeDispatchInfo）与 GPU 剔除 |
| [skill-translation/frame-pass-assemble-zh.md](skill-translation/frame-pass-assemble-zh.md) | 渲染帧组装与 pass 消费批次 |

## 模式概览

间接渲染（indirect rendering）需要一个"常驻 GPU 的实体 id 池"：每个场景模型在池里占一个 u32 槽位（数据库分配索引），渲染时 GPU 侧按范围描述直接读取这个池做剔除与间接绘制（详见 [draw-list-guide.md](draw-list-guide.md)）。问题随之而来——id 池里的实体不能乱排，必须按"绘制管线分组"组织成若干子列表，否则一个间接绘制调用无法对应唯一管线。这就是本 crate 回答的核心问题：

- **增量 PSO key**：`SceneModelGroupKey`（材质 key + 网格 key + 光栅化状态 id）描述"哪个实体与哪个实体共用同一条渲染管线"。它不逐帧全量重算，而是由增量查询（fanout 扇出）从材质、网格、状态变更自动传播到受影响的场景模型。
- **增量 id 池**：每个 (scene, key) 分组持有一个宿主侧列表与一段 GPU 池区域。实体增删只做局部 swap-remove 与稀疏写入，容量未变时完全不动池里的其他数据；容量变了才由范围分配器整块搬迁。
- **两阶段落地**：宿主侧列表维护与分配器更新在 spawn 阶段（可放进线程），GPU 写入（重定位、整块写、稀疏写）在 render 阶段统一执行。
- **trait 抽象**：`SceneBatchBasicExtractAbility` 是"从场景提取一个渲染批次"的统一入口；host 每帧提取（GLES 路径）与 device 增量提取（indirect 路径）各自实现它。
- **扩展机制**：`GroupKeyForeignImpl` 允许扩展（宽线、宽点、文字、单元网格、occ 材质、实例化模型）用自己的 key 覆盖或补充默认 key；occ-style-draw-control 更进一步，把 key 包一层 layer 并包装整个 extractor，实现按层排序、priority 排序与顶层（TopMost）独立帧绘制。

## 核心概念

| 概念 | 定义 | 说明 |
| --- | --- | --- |
| `SceneModelGroupKey` | [scene/rendering/batch-extractor/src/default_key_logic.rs](../../scene/rendering/batch-extractor/src/default_key_logic.rs) | 场景模型的 PSO 分组 key：`Standard { material, mesh, state_id }` 或 `ForeignHash { internal, require_alpha_blend }`（扩展自持 hash） |
| `MaterialGroupKey` | 同上 | 材质侧 key：`Common { ty, require_alpha_blend }` 或 `ForeignHash` |
| `MeshGroupKey` / `AttributeMeshRenderHashKey` | 同上 | 网格侧 key：`Attribute(index_ty, topology)` 或 `ForeignHash` |
| `GroupKeyForeignImpl` | 同上 | 扩展注入点：`model` / `mesh` / `material` 三项可选的外部 key 查询 |
| `StateIntern` | [scene/rendering/gpu-base/src/state.rs](../../scene/rendering/gpu-base/src/state.rs) | `StandardModelRasterizationOverride` 值内联（interning）成 `InternedId<RasterizationStates>` 的共享查询 |
| `InternedId<T>` | [utility/interning/src/lib.rs](../../utility/interning/src/lib.rs) | 值去重后的小整数 id，可 Hash 可比较 |
| `IncrementalDeviceSceneBatchExtractor<K>` | [scene/rendering/batch-extractor/src/extractor.rs](../../scene/rendering/batch-extractor/src/extractor.rs) | 增量 device 批提取器：`scene → key → PersistSceneModelListBuffer` 的二级表 + 共享 id 池 |
| `PersistSceneModelListBuffer` | [scene/rendering/batch-extractor/src/list_buffer.rs](../../scene/rendering/batch-extractor/src/list_buffer.rs) | 单组实体的宿主侧列表（Vec + handle→下标映射 + 变更记录） |
| `SceneModelListPool<K>` | [scene/rendering/batch-extractor/src/list_pool.rs](../../scene/rendering/batch-extractor/src/list_pool.rs) | 共享 GPU id 池：`ResizableGPUBuffer<[u32]>` + 范围分配器，K 映射到池内区域 |
| `GrowableRangeAllocator<K>` | [utility/growable-range-allocator/src/lib.rs](../../utility/growable-range-allocator/src/lib.rs) | 可增长范围分配器：批量 update 产出重定位/新写/移除/扩容四类变更 |
| `ExtractorUpdate<K>` / `PoolAllocationUpdate<K>` | [extractor.rs](../../scene/rendering/batch-extractor/src/extractor.rs)、[list_pool.rs](../../scene/rendering/batch-extractor/src/list_pool.rs) | spawn 阶段产出的"待落地"变更包，render 阶段消费 |
| `SparseBufferWritesSource` | [platform/graphics/webgpu-hook-utils/src/sparse_buffer_writes.rs](../../platform/graphics/webgpu-hook-utils/src/sparse_buffer_writes.rs) | 打包好的稀疏写入源（数据 + 位置对） |
| `SceneBatchBasicExtractAbility` | [scene/rendering/gpu-base/src/batch_extraction.rs](../../scene/rendering/gpu-base/src/batch_extraction.rs) | 批提取统一入口 trait：`extract_scene_batch(scene, semantic, renderer)` |
| `SceneModelRenderBatch` / `DeviceSceneModelDrawList` | [scene/rendering/gpu-base/src/batch.rs](../../scene/rendering/gpu-base/src/batch.rs) | 提取结果：device 列表（含 `impl_select_ids` 代表实体）或 host 迭代器 |
| `SceneContentKey` | [gpu-base/src/batch_extraction.rs](../../scene/rendering/gpu-base/src/batch_extraction.rs) | 提取语义：`only_alpha_blend_objects` 可选过滤 |
| `impl_select_ids` | [gpu-base/src/batch.rs](../../scene/rendering/gpu-base/src/batch.rs) | 每个子列表的"代表实体"，用于实现选择与管线哈希 |
| `OccFlavorZLayer` / `OccSceneModelGroupKey` | [extension/occ-style-draw-control/src/lib.rs](../../extension/occ-style-draw-control/src/lib.rs) | 分层绘制的 layer 枚举与"内部 key + layer"的组合 key |
| `OccStyleOrderControlSceneBatchExtractor` | 同上 | 包装基础 extractor：按 layer 过滤排序、priority 排序稀疏写、TopMost 独立提取 |

## 分层动机与数据流

先看完整数据流，再逐层展开：

```text
数据库变化（材质 alpha、网格拓扑、状态覆盖、节点可见性、实体增删）
  └─ 增量查询（fanout 扇出 / intersect / union）
       └─ SceneModelGroupKey 逐 std model 计算，再扇出到 sm
            └─ 附 scene id（SceneModelBelongsToScene）+ 可见过滤
                 └─ (scene, key) 对进入 IncrementalDeviceSceneBatchExtractor
                      ├─ spawn 阶段（线程）：prepare_updates
                      │    ├─ 宿主列表 insert/remove（swap-remove + 变更记录）
                      │    └─ 范围分配器批量 update（新写/搬迁/移除/扩容）
                      └─ render 阶段：do_updates
                           ├─ 池内重定位（batch_self_relocate）
                           ├─ 整块数据写入（move_writes）
                           └─ 稀疏写入（entity 槽位变更）
  └─ extract_scene_batch（帧内提取）
       └─ SceneModelRenderBatch::Device(DeviceSceneModelDrawList)
            └─ gpu-indirect：按 impl_select_ids 选实现
                 └─ material + shape + state_id 哈希进 PipelineHasher → PSO
```

分层动机：

- **key 层与存储层分离**。key 的增量计算完全在查询系统内完成，提取器只消费"某个实体的 key 变了"这一结果，不关心 key 怎么来的——所以扩展只需提供新的 key 查询。
- **宿主列表与 GPU 池分离**。宿主侧列表保证提取时能拿到稳定顺序与代表实体；GPU 池只存 u32 分配索引，重排（swap-remove）不引起整块重写。
- **spawn 与 render 分离**。宿主侧计算可并行化；render 阶段只有纯 GPU 写入，与帧编码自然衔接（`GPUQueryHookStage::CreateRender`）。
- **pso key 决定子列表边界**。同一子列表内的实体共享一条间接绘制命令与同一管线，这正是 `SceneModelGroupKey` 的语义。

## 增量 PSO key 的计算

### 网格侧 key

`attribute_mesh_group_key`（[default_key_logic.rs](../../scene/rendering/batch-extractor/src/default_key_logic.rs)）把网格数据变化映射为 `AttributeMeshRenderHashKey`：`index_ty` 由索引缓冲区字节宽推断（u16/u32），`topology` 取网格拓扑。推导在 spawn 阶段完成（`use_change_to_dual_query_in_spawn_stage`，见 [query-hook-guide.md](query-hook-guide.md)），再经 `StandardModelRefAttributesMeshEntity` 反向视图 fanout 到引用它的 std model。

### 材质侧 key

`use_indirect_material_indirect_group_key` 对三类标准材质（PbrMR / PbrSG / Unlit）各读 `AlphaModeOf` 组件，映射成 `MaterialGroupKey::Common { ty, require_alpha_blend }`，再 fanout 到 std model，用 `dual_query_select` 合并（三类材质实体互斥，key 集合不重叠）。alpha blend 语义直接进入 key——透明与不透明实体永远分在不同组，`extract_scene_batch` 靠 `SceneContentKey::only_opaque_objects()` / `only_alpha_blend_objects()` 按 key 上的 `require_alpha_blend` 过滤子列表。

### 状态覆盖：StateIntern

`StandardModelRasterizationOverride` 组件（[scene/core/src/model.rs](../../scene/core/src/model.rs)）让每个 std model 可选覆盖混合、深度、模板、cull 等光栅化状态。`StateIntern`（[gpu-base/src/state.rs](../../scene/rendering/gpu-base/src/state.rs)）把"可能很大的结构体值"去重成小整数 `InternedId<RasterizationStates>`，供 key 比较与管线哈希共用：

```rust
cx.use_dual_query::<StandardModelRasterizationOverride>()
  .dual_query_filter_map(|v| v) // 只有设置过覆盖的实体有 entry
  .use_dual_query_execute_map(cx, move || {
    let mut intern = intern.make_write_holder();
    move |_, v| intern.compute_intern_id(&v)
  })
```

注意 `filter_map(|v| v)`：没有覆盖的实体不出现在该查询里，因此合成 key 时 `state_id` 是 `Option<InternedId>`——`None` 与"某个具体 id"是不同的 key 值。

### 合成与扇出到 sm

`use_scene_model_group_key`（[default_key_logic.rs](../../scene/rendering/batch-extractor/src/default_key_logic.rs)）把三路合并：

- `material`（材质实体 key 扇出后，key 为 std model）与 `mesh` 做 `dual_query_intersect`——网格可能未加载，必须取交集而不是 zip。
- 再 `dual_query_union(state_id, |(a, s)| Some((a?, s)))`：要求材质∩网格存在，状态可缺省。
- `dual_query_map` 合成 `SceneModelGroupKey::Standard`。
- 最后 `fanout(sm_ref, cx)`（`SceneModelStdModelRenderPayload` 反向视图）把 key 从 std model 扇出到引用它的**场景模型**——此后查询的 key 变为 `SceneModelEntity`。

`GroupKeyForeignImpl` 的三项（`model` / `mesh` / `material`）各通过 `dual_query_select(foreign)` 覆盖对应环节：扩展实体（如宽线、occ 材质）通过 payload 外键把 key 换成自己的 `ForeignHash`，从而与标准 key 的实体天然分桶。注意 `select` 要求两侧 key 集合互斥——扩展实体与标准实体不共享一个 std model。

### 附 scene id 与可见过滤

`use_scene_model_group_key_with_scene_id_and_visible_filter`（[default_key_logic.rs](../../scene/rendering/batch-extractor/src/default_key_logic.rs)）：

- `SceneModelBelongsToScene` 反向外键（sm → scene）与 key 做 intersect，得到 `(key, scene_id)` 对。
- 可见过滤：`use_global_node_net_visible`（[scene/core/src/node.rs](../../scene/core/src/node.rs)，节点树可见性）fanout 到 sm，与 `SceneModelVisible` 组件 intersect，`filter_map` 出可见集，再用 `dual_query_filter_by_set(visible_scene_models)` 过滤 key 查询。

最终得到 `(SceneModelGroupKey, RawEntityHandle)`（key 为 sm）的增量查询，即提取器的输入。

## 增量维护：spawn / render 两阶段

`use_incremental_device_scene_batch_extractor<K: CKey>`（[lib.rs](../../scene/rendering/batch-extractor/src/lib.rs)）组装整个提取器：`use_gpu_init` 创建池与提取器，`map_spawn_stage_in_thread_dual_query` 把 key 查询的变化喂给 `prepare_updates`（有 delta 时才真正上线程），render 阶段拿到 `ExtractorUpdate` 后执行 `do_updates`，最后返回 `LockReadGuardHolder<IncrementalDeviceSceneBatchExtractor<K>>`（借助 blanket impl 直接作为 `SceneBatchBasicExtractAbility` 使用）。

### PersistSceneModelListBuffer：swap-remove 与变更记录

[list_buffer.rs](../../scene/rendering/batch-extractor/src/list_buffer.rs) 的每个 buffer 是"一个 (scene, key) 组的宿主侧实体列表"：

- `insert`：push + 记录"新位置 → 实体分配索引"到 `mapping_change`。
- `remove`：swap-remove（被删元素非尾时把尾元素换过来），映射同步更新，变更记录里"删掉旧尾位置、写下新位置"。
- `updates`（`PersistSceneModelListBufferMutation`）累积本次帧内所有槽位变更，`into_sparse_update(base_offset)` 把变更打包成带池偏移的 `SparseBufferWritesSource`。

swap-remove 的意义：单实体增删只产生 O(1) 槽位变化，GPU 侧只需稀疏写入，不必重写整组。

### SceneModelListPool 与范围分配器

[list_pool.rs](../../scene/rendering/batch-extractor/src/list_pool.rs) 持有 `ResizableGPUBuffer<AbstractReadonlyStorageBuffer<[u32]>>` 与 `GrowableRangeAllocator<K>`（[utility/growable-range-allocator](../../utility/growable-range-allocator/src/lib.rs)，xalloc TLSF 封装，K → (size, offset)）。池初始化容量 1024，对齐要求取 `min_storage_buffer_offset_alignment.max(min_uniform_buffer_offset_alignment) / 4`（u32 单位）——每个子列表区域起点必须对齐，这样各区域才能切成合法的 buffer view。

### prepare_updates 流程

[extractor.rs](../../scene/rendering/batch-extractor/src/extractor.rs) 的 `prepare_updates`（spawn 阶段，消费 `ValueChange<GroupKeyWithSceneHandle<K>>`）：

- 遍历 delta：`old_value` 从旧组移除、`new_value` 插入新组（`get_or_create` 懒建组），并记录变更前容量。
- 对每个变更过的组：容量归零则整组删除（`remove_empty`）；否则以 `new_size.next_power_of_two().max(min_size_round_up)` 作为目标容量，与旧容量比较——**容量未变时走稀疏写路径，不进分配器**；容量变了才进 `changed_groups`。
- `pool.prepare_pool_update` 批量更新分配器：产出重定位（`data_movements`）、新写、移除与扩容（`resize_to`）四类变更；实体槽位变更此时已带上池偏移打包成 `sparse_writes`。
- 若需扩容，spawn 阶段即调用 `update_pool_size`（`ResizableGPUBuffer::resize`）。

注释标记"the code here is cold path"：整块数据重写只在容量变化时发生，正常运行期的增删只走热路径稀疏写。

### do_updates：GPU 落地

[list_pool.rs](../../scene/rendering/batch-extractor/src/list_pool.rs) 的 `apply_pool_update`（render 阶段，`CreateRender` 的 encoder 上）：

- 有重定位时：`batch_self_relocate` 在独立 encoder 里先整体搬迁池内数据（old_offset → new_offset）。
- `move_writes`（整组新数据）与 `sparse_writes`（槽位级变更）直接写入池 buffer。

三者的顺序依赖正确性：先搬迁老数据，再写新组数据，最后写稀疏槽位。

## 提取：extract_scene_batch 与 DeviceSceneModelDrawList

`IncrementalDeviceSceneBatchExtractor<SceneModelGroupKey>` 的 `extract_scene_batch`（[extractor.rs](../../scene/rendering/batch-extractor/src/extractor.rs)）：

- 按 `semantic.only_alpha_blend_objects` 过滤组（None 表示不过滤）。
- 每组的代表实体（`host.first()`，即组内第一个实体）进入 `impl_select_ids`。
- 从分配器读出每组的 `(capacity, offset)` 组成 `CapacityRange`，`prepare_gpu_sub_list_ranges`（[shader/draw-list/src/lib.rs](../../shader/draw-list/src/lib.rs)）生成 GPU 侧 `StorageSubListRangeInfo`（offset / count / 前缀和），包成 `DeviceMultiRangeDispatchInfo`（[shader/draw-list/src/multi_range.rs](../../shader/draw-list/src/multi_range.rs)）。
- 组装 `DeviceDrawList { id_pool, dispatch_info: MultiRangeDispatchInfo { device_ranges, host_capacity_ranges, total_capacity } }` 与 `impl_select_ids`，返回 `SceneModelRenderBatch::Device(Some(..))`；空场景返回 `Device(None)`（GPU 层不允许零长 buffer）。

下游消费在 [scene/rendering/gpu-indirect/src/scene.rs](../../scene/rendering/gpu-indirect/src/scene.rs) 的 `use_make_scene_batch_pass_content`：按 `impl_select_ids` 的 `get_impl_distinguish_key_by_impl_select_id` 把子列表按实现分类，`use_create_or_update_indirect_draw_providers` 为每类构建间接绘制 provider（draw command 生成 + 剔除）。

## 从 group key 到 PSO：管线哈希的汇合点

group key 是"PSO 分桶"的主机侧投影，最终管线哈希在渲染实现里汇合。以标准模型为例，[std_model.rs](../../scene/rendering/gpu-indirect/src/std_model.rs) 的 `hash_shader_group_key`：

```rust
self.materials.hash_shader_group_key_with_self_type_info(model, hasher)?;
self.shapes.hash_shader_group_key_with_self_type_info(model, hasher)?;
self.states.get_gpu(model)?.hash_pipeline(hasher);
```

其中 `StateGPUImpl::hash_pipeline` 哈希的正是 `state_id: InternedId<RasterizationStates>`（[gpu-base/src/state.rs](../../scene/rendering/gpu-base/src/state.rs)），`build` 阶段用同一份 interned 状态写混合/深度/模板等 pass 状态。三者一致 ⇒ key 相同 ⇒ 同一子列表共享同一条管线。这解释了 key 设计的两个细节：

- `state_id` 必须进 key：状态覆盖改变时该组需要换管线，若不换会画错混合/深度。
- intern 而非直接放结构体：`RasterizationStates` 是大结构体，直接比较/哈希代价高；`InternedId` 是 usize，key 的 Eq/Hash 保持廉价，且哈希值与管线哈希用的是同一个 id。

## 扩展机制：GroupKeyForeignImpl 与 ForeignHash

扩展接入点有两处。key 层：提供 `model` / `mesh` / `material` 三个可选查询，通过 `dual_query_select` 覆盖默认 key 的对应环节（见上文的合成流程）。存储层：直接复用 `SceneModelListPool` / `PersistSceneModelListBuffer`（`IncrementalDeviceSceneBatchExtractor` 对 K 完全泛型），或自建提取器包装。

各扩展的实际实现（全部是"payload 外键 + fast_hash_scope 组合"的同一套路）：

| 扩展 | key 函数 | 位置 | hash 内容 |
| --- | --- | --- | --- |
| 宽线 | `use_wide_line_group_key` | [extension/wide-line/src/indirect_draw.rs](../../extension/wide-line/src/indirect_draw.rs) | `TypeId::of::<WideLineModelEntity>()` + depth enable + transparent + 线宽特判 |
| 宽点 | `use_wide_styled_points_group_key` | [extension/wide-styled-points/src/indirect_draw.rs](../../extension/wide-styled-points/src/indirect_draw.rs) | 同套路 |
| 文字 | `use_text3d_group_key` | [extension/text-3d/src/lib.rs](../../extension/text-3d/src/lib.rs) | 同套路 |
| 单元网格 | `use_cell_mesh_group_key` | [extension/cell-mesh/src/indirect_draw.rs](../../extension/cell-mesh/src/indirect_draw.rs) | `TypeId::of::<CellMeshEntity>()`（mesh 槽） |
| occ 材质 | `use_occ_material_indirect_group_key` | [extension/occ-style-material/src/indirect.rs](../../extension/occ-style-material/src/indirect.rs) | `TypeId::of::<OccStyleMaterialEntity>()` + shade type + state override（material 槽），alpha 取自 override blend |
| 实例化模型 | `use_transform_instanced_model_group_key` | [extension/transform-instanced-model/src/indirect_draw/mod.rs](../../extension/transform-instanced-model/src/indirect_draw/mod.rs) | 把内部 key 整体 hash 进 `ForeignHash`（model 槽），alpha 透传 |

`ForeignHash { internal, require_alpha_blend }` 的 `internal` 用 `fast_hash_scope`（[fast_hash_collection](../../utility/fast-hash-collection/src/lib.rs)）组合 `TypeId` 与相关组件值。要点：`TypeId` 保证不同扩展类型永不撞桶；`require_alpha_blend` 显式透传，使透明过滤（`SceneContentKey`）对扩展同样成立。

## occ-style-draw-control 扩展

[extension/occ-style-draw-control](../../extension/occ-style-draw-control/src/lib.rs) 在整套机制之上实现"分层绘制控制"：为 SceneModelEntity 增加 layer（`OccFlavorZLayer`：BotOSD / Default / Top / TopMost / TopOSD）与 priority 两个组件，使组内实体可按层排序、按 priority 排序，并把 TopMost 层抽成独立帧绘制。

### key 扩展

`use_scene_model_occ_group_key` 把内部 `(SceneModelGroupKey, scene_id)` 与 `SceneModelOccStyleLayer` intersect，合成 `OccSceneModelGroupKey { internal, layer }`——layer 进 key 意味着不同层天然分桶，提取时无需再按层拆分。

### extractor 扩展

`use_occ_incremental_device_scene_batch_extractor` 用 `SceneModelListPool<(RawEntityHandle, OccSceneModelGroupKey)>` 构造 `IncrementalDeviceSceneBatchExtractor`，包进 `OccStyleOrderControlSceneBatchExtractor`（复用基础 extractor 的全部维护逻辑）。变化处理与基础版的差异在于：key 查询 `join(priority_changes)` 后 `map_spawn_stage_in_thread`，只要任一侧有 delta 就执行 `prepare_updates(c1, c2.delta())`——priority 变化也要触发重排。

`extract_scene_batch` 在基础提取之上：过滤掉 `TopMost` 层、按 `layer as u32` 排序组装 `DeviceSceneModelDrawList`，其余（代表实体、容量范围、GPU 范围）与基础版一致。`get_top_most_layer` 反向取出只含 `TopMost` 的批次。

### priority 排序与稀疏写

`prepare_updates`（[lib.rs](../../extension/occ-style-draw-control/src/lib.rs)）：

- 先调 `internal.prepare_updates(delta)` 得到基础变更（key 变化导致的增删已包含在内）。
- 遍历 `priority_changes`：对每个 priority 变化的 sm，从查询视图取回其 (key, scene) 补进 `changed_keys`（视图里没有说明该 sm 被可见过滤掉了）。
- 对每个变更组调用 `sort_by_priority` 生成排序写入，并带上池偏移打包成 `sort_sparse_writes`，与基础 `pool_update` 一起返回。

注意当前 `sort_by_priority` 只是占位：真正的按 priority 排序被 `todo` 注释（代码注释坦诚指出实现有问题），现在只按 host 列表差异生成稀疏写入骨架，机制上预留了排序与写入的位置。`do_updates` 先应用基础池更新，再写排序稀疏写。

### TopMost 独立帧

[application/viewer-content-api/src/top_most_standalone_draw.rs](../../application/viewer-content-api/src/top_most_standalone_draw.rs) 的 `TopMostStandaloneDraw`（`ViewerFrameRenderingExtension`）在 post frame 阶段通过 `as_any().downcast_ref` 拿到具体提取器，调用 `get_top_most_layer`，用独立 attachment + pass（`scene_top_most_mass`）绘制后以预乘 alpha 混合拷回主目标（渲染侧的 MSAA resolve 与拷贝细节见 [viewer-content-api-guide.md](viewer-content-api-guide.md)）。GLES 路径则走 `OccStyleOrderControlSceneBatchExtractorGles::get_top_most_layer`。

### GLES（host）变体

[gles.rs](../../extension/occ-style-draw-control/src/gles.rs) 面向 host 驱动路径：`use_occ_host_scene_batch_extractor` 包住 `DefaultSceneBatchExtractor`（[gpu-base/src/batch_extraction.rs](../../scene/rendering/gpu-base/src/batch_extraction.rs) 的每帧全量 host 提取），加上 layer / priority 读视图。提取时过滤 TopMost、按 `(layer << 32) | priority` 排序，产出 `IteratorAsHostRenderBatch`；若 renderer 提供 `indirect_batch_direct_creator`（非 host-driven 模式，即间接后端且未开启 host-driven 时 `IndirectSceneRenderer` 提供该 creator；host-driven 模式下它返回 `None`，两种路径的互斥见 [gpu-indirect-batch-collector-guide.md](gpu-indirect-batch-collector-guide.md) 的「常见疑问」），则现场分类成 device 批次（[gpu-indirect/src/scene.rs](../../scene/rendering/gpu-indirect/src/scene.rs) 的 `create_batch_from_iter`）。

## 使用模板

### 模板一：在 viewer 中装配 occ 增量提取

[application/viewer-content/src/rendering/frame_all.rs](../../application/viewer-content/src/rendering/frame_all.rs) 的 indirect 分支展示了完整装配链：

```rust
let mesh_key = attribute_mesh_group_key(cx, mesh_changes__);
cx.scope(|cx| {
  let wide_line_key = use_wide_line_group_key(cx, use_native_line_for_one_width_line);
  let wide_point_key = use_wide_styled_points_group_key(cx);
  let text_key = use_text3d_group_key(cx);
  let impl_key = wide_line_key
    .dual_query_select(wide_point_key).dual_query_boxed()
    .dual_query_select(text_key).dual_query_boxed();

  let occ_material = rendiation_occ_style_material::indirect::use_occ_material_indirect_group_key(cx);
  let cell_mesh = use_cell_mesh_group_key(cx);

  let key_impl = GroupKeyForeignImpl {
    model: Some(impl_key),
    material: Some(occ_material),
    mesh: Some(cell_mesh),
  };

  let internal = use_scene_model_group_key(cx, key_impl, mesh_key);
  // （可选）实例化模型包一层：use_transform_instanced_model_group_key
  let internal = use_scene_model_group_key_with_scene_id_and_visible_filter(cx, internal);
  let sm_group_key = use_scene_model_occ_group_key(cx, internal);
  indirect_extractor = use_occ_incremental_device_scene_batch_extractor(cx, sm_group_key);
})
```

### 模板二：为新的扩展模型接入分组

新扩展只需提供 payload 外键 + 组件值 → `ForeignHash` 的 key 查询（模式见上文扩展表），在 `GroupKeyForeignImpl` 对应槽位接入。注意 select 的互斥约束：扩展实体与标准实体不得共用同一个 key 查询的 key 集合。若扩展还要控制层/顺序（如 UI 层、叠加层），照 occ 的模式把 layer/priority 组件并进 key、包一层 extractor 即可。

### 模板三：帧内消费批次

```rust
// frame_viewport.rs 的 ViewerSceneRenderer 持有 batch_extractor
let all_opaque = renderer.batch_extractor.extract_scene_batch(
  scene,
  SceneContentKey::only_opaque_objects(),
  renderer.scene,
);
let all_transparent = renderer.batch_extractor.extract_scene_batch(
  scene,
  SceneContentKey::only_alpha_blend_objects(),
  renderer.scene,
);
// 见 application/viewer-content/src/rendering/lighting/light_pass/mod.rs
```

帧组装处（[frame_viewport.rs](../../application/viewer-content/src/rendering/frame_viewport.rs)）把 `batch_extractor` 放进 `ViewerSceneRenderer`，光照 pass 提取 opaque/transparent 两批分别绘制。`ViewerBatchExtractor`（[frame_all.rs](../../application/viewer-content/src/rendering/frame_all.rs)）在 device 与 host 提取器之间选择：indirect 渲染优先用增量 device 提取器，否则回退 host。

## 延伸阅读

- 增量查询组合子语义（select/intersect/union/filter_by_set/fanout）：[utility/query/src/delta_query/mod.rs](../../utility/query/src/delta_query/mod.rs)
- 两阶段任务调度与 UseResult：[utility/query-hook/src/use_result.rs](../../utility/query-hook/src/use_result.rs)
- 范围分配器与批量变更：[utility/growable-range-allocator/src/lib.rs](../../utility/growable-range-allocator/src/lib.rs)
- 稀疏写入源：[platform/graphics/webgpu-hook-utils/src/sparse_buffer_writes.rs](../../platform/graphics/webgpu-hook-utils/src/sparse_buffer_writes.rs)
- GPU 绘制列表与多范围派发：[shader/draw-list/src/lib.rs](../../shader/draw-list/src/lib.rs)、[shader/draw-list/src/multi_range.rs](../../shader/draw-list/src/multi_range.rs)
- 管线哈希与状态覆盖：[scene/rendering/gpu-base/src/state.rs](../../scene/rendering/gpu-base/src/state.rs)、[scene/rendering/gpu-indirect/src/std_model.rs](../../scene/rendering/gpu-indirect/src/std_model.rs)
