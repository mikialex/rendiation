# Rendiation 属性网格间接渲染指南（scene/rendering/gpu-indirect/shape/attribute）

本文梳理 [scene/rendering/gpu-indirect/src/shape/attribute](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs) 的实现：标准模型（StandardModel）引用的属性网格（AttributesMesh）如何把顶点/索引数据以"网格级分配"的方式放进 GPU 常驻缓冲池，用一张 `AttributeMeshMeta` 元数据表做间接寻址，并由 GPU 上的 compute 派发逐实体生成间接绘制命令。它是 rendiation 间接渲染路径（batch extractor → 间接绘制命令 → 光栅化）中"网格形状"这一环节的完整实现，也是 attribute-mesh-lod、transform-instanced-model 等扩展的基座。

## 前置阅读

| 文档 | 内容 |
| --- | --- |
| [skill-translation/shader-edsl-core-zh.md](skill-translation/shader-edsl-core-zh.md) | Node 表达式、shader 结构体、GPU 侧索引加载（load / index / sized_ty） |
| [skill-translation/shader-edsl-graphics-zh.md](skill-translation/shader-edsl-graphics-zh.md) | GraphicsShaderProvider、顶点/片元阶段、语义注册 |
| [skill-translation/shader-edsl-binding-and-typed-container-zh.md](skill-translation/shader-edsl-binding-and-typed-container-zh.md) | bind_by / bind、StorageBufferReadonlyDataView 等 GPU 资源容器 |
| [skill-translation/fundamental-gpu-component-model-zh.md](skill-translation/fundamental-gpu-component-model-zh.md) | RenderComponent / ShaderHashProvider / ShaderPassBuilder 组件模型 |
| [skill-translation/scene-core-structure-zh.md](skill-translation/scene-core-structure-zh.md) | SceneModelEntity / StandardModelEntity、payload 外键、节点变换 |
| [draw-list-guide.md](draw-list-guide.md) | DeviceDrawList、多范围绘制与 GPU 剔除（间接绘制的 id 池基础） |
| [batch-extractor-guide.md](batch-extractor-guide.md) | 增量 PSO key 与 id 池分桶（间接绘制的批次来源） |

间接绘制命令的 trait 抽象（`NoneIndexedDrawCommandBuilder` / `IndirectDrawProvider` / `DrawCommandBuilder` 等）位于 [scene/rendering/gpu-base/src/mid/](../../scene/rendering/gpu-base/src/mid/mod.rs)，其完整机制见 [indirect-draw-command-guide.md](indirect-draw-command-guide.md)；本指南只说明 attribute mesh 如何实现与消费这些 trait，不再展开 mid 层本身。

## 模式概览

普通（host 侧直接提交）渲染把顶点缓冲绑定为 vertex buffer，绘制时 GPU 硬件按 `vertex_index` 取数。间接渲染做不到这一点：场景模型数量巨大，每个网格都绑一次 buffer 既不现实也没必要。attribute mesh 的方案是把所有网格的顶点/索引数据**按网格切片**拼进两个常驻缓冲池（顶点池、索引池），再用一张元数据表记录"每个网格的数据在池里的哪一段"：

- 网格数据**池化**：顶点池按 `[Vec3 position, Vec3 normal, Vec2 uv]` 的紧凑布局连续存放，索引池按网格切段；段落在池内的位置由 `GrowableRangeAllocator` 分配，段起点就是"GPU 地址"。
- **两跳间接寻址**：场景模型（sm）→ 网格（mesh）→ 元数据（`AttributeMeshMeta`）→ 池内偏移。sm 与 mesh 各有一个 u32 映射表（`sm_to_mesh_device`、mesh 槽位的元数据表），sm 的分配索引经过一跳得到 mesh 的分配索引，再经一跳得到池内偏移。
- **宿主与设备双通道**：元数据表同时维护 GPU 副本与宿主备份。宿主备份供 host-driven（GLES）路径现场生成 `DrawCommand`（每个实体一个 command）；GPU 副本供 compute 着色器按 sm id 批量生成间接绘制参数。
- **一套 creator 两个形态**：`AttributeMeshIndirectDrawCreator` 同时实现 indexed 与 none-indexed 两套 builder；索引还是非索引由网格是否有索引数据决定，直接决定子列表走 `MultiIndirectDrawCount`（索引）还是 `MultiIndirectDraw`（数组）。
- **标记值约定**：分配失败用 `DEVICE_RANGE_ALLOCATE_FAIL_MARKER = u32::MAX` 标记（[webgpu-hook-utils/src/allocator.rs:70](../../platform/graphics/webgpu-hook-utils/src/allocator.rs#L70)）。宿主侧看到标记返回 `None`（跳过该实体）；设备侧范围分配保证失败段 count 为 0，空绘制命令自然无效果。

## 核心概念

| 概念 | 定义 | 说明 |
| --- | --- | --- |
| `AttributeMeshMeta` | [shape/attribute/mod.rs:498](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L498) | 每个网格一条的池内地址记录：索引段 `(index_offset, count, is_u16_indices, is_u16_indices_padded)` 与三个顶点属性段 `(offset, count)` |
| `AttributeMeshIndirectRenderer` | [shape/attribute/mod.rs:516](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L516) | 整个网格系统的渲染器：顶点池、索引池、元数据表、sm→mesh 映射与若干宿主查询视图 |
| `AttributeMeshIndirectDrawCreator` | [shape/attribute/draw_cmd.rs:3](../../scene/rendering/gpu-indirect/src/shape/attribute/draw_cmd.rs#L3) | 间接绘制命令生成器：宿主侧读元数据备份产出 `DrawCommand`，GPU 侧构建 invocation 产出 `DrawIndirectArgsStorage` |
| `AttributeMeshIndirectDrawCreatorInvocation` | [draw_cmd.rs:108](../../scene/rendering/gpu-indirect/src/shape/attribute/draw_cmd.rs#L108) | 上述 creator 的 compute 侧实现：`generate_draw_command(draw_id)` 逐 sm 生成参数 |
| `AttributeMeshIndirectDispatcher` | [shape/attribute/render.rs:6](../../scene/rendering/gpu-indirect/src/shape/attribute/render.rs#L6) | 光栅化侧的 GPU 资源分发器：元数据表 + 顶点池的绑定与取数逻辑 |
| `AttributeMeshIndirectRasterDispatcher` | [render.rs:21](../../scene/rendering/gpu-indirect/src/shape/attribute/render.rs#L21) | 包装 dispatcher 的形状组件：加上 topology 与索引类型，实现 `ShaderPassBuilder` / `GraphicsShaderProvider` |
| `IndirectAttributeMeshInitConfig` | [mod.rs:17](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L17) | 池的初始/最大容量（索引数与顶点 u32 数）与法线量化开关 |
| `DEVICE_RANGE_ALLOCATE_FAIL_MARKER` | [allocator.rs:70](../../platform/graphics/webgpu-hook-utils/src/allocator.rs#L70) | `u32::MAX`：分配失败的标记值 |
| `used_in_midc_downgrade` | [draw_cmd.rs:10](../../scene/rendering/gpu-indirect/src/shape/attribute/draw_cmd.rs#L10) | 是否运行在 MIDC 降级模式（见下文），影响索引基址的单位与管线哈希 |

## 分层动机与数据流

先看完整数据流，再逐层展开：

```text
场景数据库:AttributesMeshEntity(+Topology) / VertexBufferRelation(+Semantic) / 索引与顶点 BufferEntity
  └─ viewer_mesh_input → AttributesMeshDataChangeInput(UriLoadResult 包装的网格数据)
       └─ create_sub_buffer_changes_from_mesh_changes (scene/core/src/mesh.rs)
            ├─ 顶点数据源 (relation, semantic) → 顶点池分配器(GrowableRangeAllocator)
            │    每个属性段写入池,并稀疏写回 AttributeMeshMeta.position/normal/uv_offset+count
            └─ 索引数据源 → 索引池分配器
                 索引字节宽 → IndexFormat(u16/u32);计数与对齐标志写回 meta.is_u16_indices(+padded)
  └─ AttributeMeshMeta 元数据表(GPU storage buffer + 宿主备份,mesh 分配索引为下标)
  └─ sm_to_mesh_device 映射表(sm 分配索引 → mesh 分配索引,由 std model→mesh 反向外键 fanout)
       └─ AttributeMeshIndirectRenderer
            ├─ IndirectDrawProviderCreator:use_create_or_update_indirect_draw_providers
            │    └─ make_draw_command_builder → Indexed/NoneIndexed 包装 DrawCommandBuilder
            │         └─ use_and_create_default_indirect_draw_provider (gpu-base/mid)
            │              └─ compute 派发:invocation.generate_draw_command(sm id)
            │                   ├─ 索引: meta.count ×2(或 ×2-1),base_index(×2 当 u16 且非降级)
            │                   └─ 非索引: meta.position_count / 3
            │              └─ MultiIndirectDrawBatch / MIDCDowngradeBatch → IndirectDrawProvider
            ├─ IndirectModelShapeRenderImpl::make_component_indirect
            │    └─ AttributeMeshIndirectRasterDispatcher(顶点着色器取数组件)
            └─ hash_shader_group_key: topology + indices_ty
  └─ 渲染帧(scene.rs):按 impl 区分 key 分组 → 每类建 provider → render_indirect_batch_models
       顶点阶段: sm id → (std model 注入器) → mesh id → meta → 池内偏移 → position/normal/uv
```

分层动机：

- **数据层与寻址层分离**。网格数据按网格切片进池，谁画什么只取决于元数据表里的偏移；网格增删/换数据只引起该网格对应段的分配器变更，不影响其他网格与整个寻址结构。
- **宿主侧与设备侧分离**。宿主侧画命令（host-driven / GLES）读的是元数据表的宿主备份；设备侧（间接）读 GPU 副本。同一份 `AttributeMeshMeta` 布局两头共享，保证两侧画的是同一段数据。
- **与 mid 层抽象松耦合**。renderer 只负责产出 `DrawCommandBuilder`（indexed 或 none-indexed），命令池的 compute 派发、子列表切分、剔除、midc 降级全部由 [gpu-base/src/mid/mod.rs](../../scene/rendering/gpu-base/src/mid/mod.rs) 的通用逻辑完成——attribute mesh 只需描述"每个实体画多少、从池里哪段画"。

## 数据模型与上传管线

### 网格实体模型

属性网格在 [scene/core/src/mesh.rs:3](../../scene/core/src/mesh.rs#L3) 建模为三类实体：

- `AttributesMeshEntity`：网格本体，携带 `AttributesMeshEntityTopology`（绘制拓扑）；索引数据通过 `AttributeIndexRef`（`SceneBufferView`）挂在它名下。
- `AttributesMeshEntityVertexBufferRelation`：顶点属性视图，携带 `AttributesMeshEntityVertexBufferSemantic`（Position / Normal / TexCoords 等）与反向外键 `RefAttributesMeshEntity`（[mesh.rs:38](../../scene/core/src/mesh.rs#L38)）；实际顶点字节经 `AttributeVertexRef` 指向 `BufferEntity`。
- `BufferEntity` + `BufferEntityData`：真正存字节的缓冲实体。

`create_sub_buffer_changes_from_mesh_changes`（[mesh.rs:283](../../scene/core/src/mesh.rs#L283)）把 `AttributesMeshDataChangeInput`（按 mesh 聚合的数据变化，[mesh.rs:336](../../scene/core/src/mesh.rs#L336)）拆成两个数据源：顶点数据源 `(relation, semantic)` 与索引数据源 `(mesh)`。它内部维护一张 `vertex_mapping`（mesh → 其 vertex relation 列表），mesh 移除时能级联找到该 mesh 的所有顶点段并一并标为移除——**顶点以 relation 为单位分配，生命周期跟随 mesh**。

### 池与元数据表

`use_attribute_mesh_indirect_renderer`（[mod.rs:42](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L42)）是系统入口，做四件事：

- **索引池**（`use_attribute_indices_updates`，[mod.rs:193](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L193)）：`AbstractReadonlyStorageBuffer<[u32]>` + `GrowableRangeAllocator`，按 `(mesh, byte 大小)` 分配段。u16 索引且字节数非 4 的倍数时尾部补零凑整（上传路径要求 4 字节对齐块，[mod.rs:253](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L253)）。
- **顶点池**（`use_attribute_vertex_updates`，[mod.rs:305](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L305)）：同为分配器 + 池，按 `(relation, 字节大小)` 分配。若开启 `enable_normal_quantization_convert`，Normal 语义的 `Vec3<f32>` 会在 spawn 阶段经 octahedral 编码压成 u32 再上传（[mod.rs:373](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L373)）。
- **元数据表**：`use_storage_buffer_with_host_backup::<AttributeMeshMeta>`（[mod.rs:91](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L91)）创建 GPU storage buffer 与宿主备份，**以 mesh 分配索引为下标**（`use_max_item_count_by_db_entity::<AttributesMeshEntity>`）。各段分配结果通过 `update_storage_array_with_host` 稀疏写回对应字段：索引段写 `is_u16_indices` 标志（[mod.rs:109](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L109)）与 `index_offset`（[mod.rs:122](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L122)）；顶点段由 `write_field_offset`（[mod.rs:487](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L487)）按语义映射到 `position/normal/uv_offset` 字段。注意同一网格三个属性的 count 被假定相同（见 [mod.rs:497](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L497) 的注释）。
- **sm → mesh 映射表**：`sm_to_mesh_device: use_storage_buffer::<u32>`（[mod.rs:128](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L128)），以 sm 分配索引为下标。数据来自 `StandardModelRefAttributesMeshEntity`（std model → mesh 外键）经 `SceneModelStdModelRenderPayload` 反向视图 fanout 到 sm（[mod.rs:131](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L131)）。mesh 实体的分配索引存 0，**没有网格的 sm 存 `u32::MAX`**，GPU 侧读到此值即为无效（`shader_assert` 被注释掉的意图）。

`AttributeMeshMeta` 本身（[mod.rs:498](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L498)）是 `#[repr(C)] + std430 + ShaderStruct` 的 shader 结构体，宿主与设备共用同一内存布局：

```rust
pub struct AttributeMeshMeta {
  pub index_offset: u32,          // 索引段在索引池的起点(u32 槽位)
  pub count: u32,                 // 索引段长度(u32 槽位数,u16 索引时两个/槽)
  pub is_u16_indices: Bool,       // 索引字节宽
  pub is_u16_indices_padded: Bool,// u16 且尾部补零(末槽只有一个真实索引)
  pub position_offset: u32, pub position_count: u32,
  pub normal_offset: u32,   pub normal_count: u32,
  pub uv_offset: u32,       pub uv_count: u32,
}
```

`position_count` 等 count 是**标量（f32）数量**而非顶点数——三个浮点一个顶点，故顶点数 = `position_count / 3`。

## 宿主侧画命令：AttributeMeshIndirectDrawCreator

`AttributeMeshIndirectDrawCreator`（[draw_cmd.rs:3](../../scene/rendering/gpu-indirect/src/shape/attribute/draw_cmd.rs#L3)）是"网格 → 画命令"的转换器，持有元数据表（GPU + 宿主备份）、sm→mesh 查询与设备表、`used_in_midc_downgrade` 标志。

宿主侧 `draw_command_host_access`（[draw_cmd.rs:13](../../scene/rendering/gpu-indirect/src/shape/attribute/draw_cmd.rs#L13)，none-indexed 版本）：

- `sm_to_mesh.access(sm)` 拿到 mesh id，在宿主备份里按 mesh 分配索引读 `AttributeMeshMeta`。
- 分配失败标记 → 返回 `None`，该实体跳过不画。
- 否则产出 `DrawCommand::Array { vertices: 0..position_count/3, instances: 0..1 }`。

indexed 版本（[draw_cmd.rs:50](../../scene/rendering/gpu-indirect/src/shape/attribute/draw_cmd.rs#L50)）多一步 u16 展开：`count` 是 u32 槽位数，真实索引数在 u16 时为 `count * 2`，若补零尾槽则 `count * 2 - 1`（[draw_cmd.rs:64](../../scene/rendering/gpu-indirect/src/shape/attribute/draw_cmd.rs#L64)）；产出 `DrawCommand::Indexed { base_vertex: 0, indices: start..end, instances: 0..1 }`。

`build_invocation`（[draw_cmd.rs:31](../../scene/rendering/gpu-indirect/src/shape/attribute/draw_cmd.rs#L31)）把 metadata 与 sm_to_mesh_device 经 `bind_by` 转成 shader 指针，构造 `AttributeMeshIndirectDrawCreatorInvocation`；`bind`（[draw_cmd.rs:44](../../scene/rendering/gpu-indirect/src/shape/attribute/draw_cmd.rs#L44)）在宿主侧绑定同两份资源。**同一 buffer 在宿主侧做绑定、在 shader 里做指针读取，是 EDSL 绑定系统的常见模式**。管线哈希只含 `used_in_midc_downgrade`（[draw_cmd.rs:101](../../scene/rendering/gpu-indirect/src/shape/attribute/draw_cmd.rs#L101)），索引类型不参与哈希——它只改变参数编码，不改变计算逻辑本身。

### 设备侧 invocation

`generate_draw_command(draw_id)`（`draw_id` 即 sm id）在 compute 中执行两跳寻址：

- `mesh_handle = sm_to_mesh_device[draw_id]`，`meta = metadata[mesh_handle]`。
- none-indexed（[draw_cmd.rs:155](../../scene/rendering/gpu-indirect/src/shape/attribute/draw_cmd.rs#L155)）：`vertex_count = position_count / 3`（注释说明范围分配保证失败时 count 为 0），`base_instance = draw_id`。
- indexed（[draw_cmd.rs:114](../../scene/rendering/gpu-indirect/src/shape/attribute/draw_cmd.rs#L114)）：`vertex_count = count`，u16 时 `×2`、补零尾槽再 `-1`；`base_index` 的单位取决于 `used_in_midc_downgrade`——降级模式下索引池以 u32 在设备端读取，基址直接是 u32 槽位；原生 MIDC 绘制索引缓冲以 u16 绑定，基址要 `×2`（[draw_cmd.rs:136](../../scene/rendering/gpu-indirect/src/shape/attribute/draw_cmd.rs#L136)）。

生成出的命令写入命令池（由 mid 层 `use_and_create_default_indirect_draw_provider` 完成，见 [gpu-base/src/mid/mod.rs:84](../../scene/rendering/gpu-base/src/mid/mod.rs#L84)），最终以 `MultiIndirectDrawCount`（索引）或 `MultiIndirectDraw`（数组）间接派发。

## 顶点着色器取数：两跳寻址的下半程

光栅化侧的组件是 `AttributeMeshIndirectRasterDispatcher`（[render.rs:21](../../scene/rendering/gpu-indirect/src/shape/attribute/render.rs#L21)）：内部持有 `AttributeMeshIndirectDispatcher`（元数据表 + 顶点池 + 索引池 + 量化开关）与 `topology` / `indices_ty`。

- `ShaderPassBuilder::setup_pass`（[render.rs:37](../../scene/rendering/gpu-indirect/src/shape/attribute/render.rs#L37)）：有索引时把索引池以 `u16/u32` 格式设为索引缓冲（`set_index_buffer_by_buffer_resource_view`，注意池带有 storage usage 故需取 view），再绑定元数据表与顶点池（`bind_base_invocation`）。
- `GraphicsShaderProvider::build`（[render.rs:54](../../scene/rendering/gpu-indirect/src/shape/attribute/render.rs#L54)）：顶点阶段查询 `IndirectAbstractMeshId`（mesh 分配索引，由 std model 注入器注册，见下）与内建 `VertexIndex`，调用 `get_position_normal_uv` 取三个属性，注册 `GeometryPosition/Normal/UV`，最后写 `topology`。

`IndirectAttributeMeshDispatcherBaseInvocation`（[render.rs:74](../../scene/rendering/gpu-indirect/src/shape/attribute/render.rs#L74)）是取数的核心，全部用 `load_from_u32_buffer` 从顶点池**按紧凑布局（Packed）手工加载**：

- `get_position`（[render.rs:113](../../scene/rendering/gpu-indirect/src/shape/attribute/render.rs#L113)）：`Vec3<f32>` 从 `position_offset + vertex_id * 3` 处加载。
- `get_normal`（[render.rs:92](../../scene/rendering/gpu-indirect/src/shape/attribute/render.rs#L92)）：`normal_offset == u32::MAX`（无该属性）时分支返回零向量；开启量化时对 u32 做 `decode_octahedral_normal_fn` 解压，否则 Packed 加载 `Vec3`。
- `get_uv`（[render.rs:125](../../scene/rendering/gpu-indirect/src/shape/attribute/render.rs#L125)）：同上，`Vec2`，无属性返回零。

注意 `mesh_handle` 与 `draw_id`（sm id）是不同实体：`draw_id` 在命令生成侧用于定位 sm 槽位，顶点侧的 `IndirectAbstractMeshId` 由 `SceneStdModelIdInjector`（[std_model.rs:406](../../scene/rendering/gpu-indirect/src/std_model.rs#L406)）从 `sm_to_std_model_device` 与 `SceneStdModelStorage.mesh`（mesh 分配索引，[std_model.rs:484](../../scene/rendering/gpu-indirect/src/std_model.rs#L484)）注入——两跳寻址的第二跳在顶点阶段完成。

## 与 mid 层抽象的衔接

`AttributeMeshIndirectRenderer` 实现三层 trait（[mod.rs:546](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L546)）：

- `IndirectDrawProviderCreator`：`get_impl_distinguish_key_by_impl_select_id` 用 `TypeId + is_indexed` 区分实现（索引类型不影响实现差异，见 [mod.rs:547](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L547)）；`use_create_or_update_indirect_draw_providers` 把 builder 交给 mid 层的 `use_and_create_default_indirect_draw_provider`（[gpu-base/src/mid/mod.rs:84](../../scene/rendering/gpu-base/src/mid/mod.rs#L84)），并透传 `used_in_midc_downgrade`。
- `DrawCommandBuilderCreator`（[mod.rs:598](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L598)）：按 `indices_ty` 是否为 `Some` 决定 `DrawCommandBuilder::Indexed` 或 `NoneIndexed`（[gpu-base/src/mid/mod.rs:12](../../scene/rendering/gpu-base/src/mid/mod.rs#L12)）。
- `IndirectModelShapeRenderImpl`（[mod.rs:611](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L611)）：`make_component_indirect` 产出光栅化组件；`get_index_storage_buffer` 暴露索引池（供 midc 降级时以 storage 读取，[mod.rs:630](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L630)）；`hash_shader_group_key` 哈希 `topology + indices_ty`（[mod.rs:644](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L644)），与渲染组件的哈希一致。

### MIDC 降级模式

`used_in_midc_downgrade = require_midc_downgrade(&gpu.info, force)`（[mod.rs:169](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L169)）。viewer 侧 force 开关同时来自 `using_host_driven_indirect_draw` 与 `using_texture_as_storage_buffer_for_indirect_rendering`（后者经 `merge_with_vertex_allocator` 参数，合并处见 [mod.rs:52](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L52)）；降级判定条件与降级管线本身见 [indirect-draw-command-guide.md](indirect-draw-command-guide.md) 的「MIDC 降级机制」，这里只说明 attribute mesh 的降级敏感点：

- **索引基址单位**：原生 MIDC 下索引缓冲以 u16 绑定、`base_index` 乘 2；降级模式下索引池经 storage 以 u32 读取、`base_index` 直接是 u32 槽位——上文的 `×2` 差异正来源于此。
- **顶点侧索引展开**：降级后成为单段无索引绘制，`MidcDowngradeWrapperForIndirectMeshSystem`（[webgpu-midc-downgrade/src/mesh_sys_wrapper.rs:6](../../platform/graphics/webgpu-midc-downgrade/src/mesh_sys_wrapper.rs#L6)）用内建 `VertexIndexForMIDCDowngradeBaseIndex` / `RelativeInSubDraw` 在设备端把索引池按 u32 读出并展开成 `VertexIndex`（u16 时一个 u32 拆两个，低半/高半按奇偶选取）。
- **索引池暴露**：`get_index_storage_buffer`（[mod.rs:630](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L630)）把索引池以 storage 形式交给降级包装器绑定。

## 用户视角：如何把 attribute mesh 装进渲染管线

### viewer 主路径（indirect）

[application/viewer-content/src/rendering/frame_all.rs:271](../../application/viewer-content/src/rendering/frame_all.rs#L271)：

```rust
let mesh = use_attribute_lod_mesh_indirect_renderer(
  cx, &init_config.indirect_attribute_mesh_init, /* … */
  init_config.using_texture_as_storage_buffer_for_indirect_rendering,
  self.using_host_driven_indirect_draw, mesh_changes, node.clone(), /* … */ );
// 形状实现与 cell_mesh 并列成 Vec<Box<dyn IndirectModelShapeRenderImpl>>
let std_model = use_viewer_std_model_renderer(cx, materials, mesh, /* … */);
// 模型实现与宽线/宽点/文字/实例化并列 → use_indirect_scene_model → IndirectSceneRenderer
```

`use_attribute_lod_mesh_indirect_renderer`（[attribute-mesh-lod/src/lib.rs:42](../../scene/rendering/attribute-mesh-lod/src/lib.rs#L42)）在基础 renderer 之上叠加 LOD：按 `LODConversionMode`（默认代码路径 `HalfCount` 计数逐级减半，viewer 配置 `ErrorDoubling` 误差逐级倍增，见 [attribute-mesh-lod-guide.md](attribute-mesh-lod-guide.md)）为每个网格生成若干 `LODLevelInfo`，`level_meta` 记录 (levels 偏移, 级别数, 根级别数)。索引网格的 builder 换成 `AttributeLODMeshIndirectDrawCreator`（[attribute-mesh-lod/src/draw_cmd.rs:4](../../scene/rendering/attribute-mesh-lod/src/draw_cmd.rs#L4)）：宿主侧只画根级别（[draw_cmd.rs:25](../../scene/rendering/attribute-mesh-lod/src/draw_cmd.rs#L25)）；设备侧按"屏幕空间投影误差 ≤ 阈值"从粗到细挑级别（投影误差 = 世界误差 × 世界缩放 × 像素/单位，[draw_cmd.rs:143](../../scene/rendering/attribute-mesh-lod/src/draw_cmd.rs#L143)），无 LOD 元数据时回退整网格。非索引网格直接透传基础 creator。

`merge_with_vertex_allocator`（对应 viewer 配置 `using_texture_as_storage_buffer_for_indirect_rendering`）强制走降级路径：纹理无法当索引缓冲绑定时，索引池改用纯分配器管理并在设备端按 u32 读取。

### 帧内消费与 host-driven 路径

帧内消费的完整流程（`use_make_scene_batch_pass_content` 按实现分类 → `use_compute_selected_sub_list_dispatch_info` → 按类建 provider → 提交）见 [gpu-indirect-batch-collector-guide.md](gpu-indirect-batch-collector-guide.md)（帧内链路组织）与 [indirect-draw-command-guide.md](indirect-draw-command-guide.md)（命令层消费），这里只强调 attribute mesh 相关的两点：

- 光栅化组件由 `IndirectModelShapeRenderImpl::make_component_indirect` 产出（见上文「与 mid 层抽象的衔接」），提交时经 `render_indirect_batch_models`（[scene_model.rs:96](../../scene/rendering/gpu-indirect/src/scene_model.rs#L96)）与材质、节点等一起按绑定槽组装，以 `RenderMethod::TraditionalDraw(command)` 提交（[scene_model.rs:136](../../scene/rendering/gpu-indirect/src/scene_model.rs#L136)）。
- host-driven（GLES / 无 device 批提取）路径见 [host_driven.rs:4](../../scene/rendering/gpu-indirect/src/host_driven.rs#L4)：对 host 批次按 shader key 分类，每组取代表实体的 `draw_command_builder` 逐实体调 `draw_command_host_access` 生成 `DrawIndexedIndirectArgsStorage` / `DrawIndirectArgsStorage` 命令，再经 host 侧 midc 降级合并为 `HostDrivenIndirectProvider`。此路径完全依赖 `AttributeMeshMeta` 的**宿主备份**，不需要元数据表的 GPU 副本参与。

## 使用要点

- 元数据表与顶点/索引池都以实体的**数据库分配索引**为下标（`use_max_item_count_by_db_entity`），宿主侧访问统一用 `alloc_index()`，不要假设为连续稠密 id。
- 画命令的顶点数来自 `position_count / 3`；u16 索引的 `count` 是 u32 槽位数而非索引数——宿主与设备两侧各展开一次，公式必须一致（`×2`，尾槽补零再 `-1`）。
- `normal/uv_offset == u32::MAX` 表示该属性不存在（非索引网格的标志位更新被刻意忽略，因为设备端不会再访问，见 [mod.rs:107](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L107) 的注释）。
- 网格数据按 `UriLoadResult` 异步加载：未加载完成（`LivingOrLoaded` 之外）的网格不进池，对应槽位保持标记值，绘制自然跳过。

## 延伸阅读

- 命令生成与 provider 抽象：[gpu-base/src/mid/mod.rs:12](../../scene/rendering/gpu-base/src/mid/mod.rs#L12)、[gpu-base/src/mid/none_indexed.rs:3](../../scene/rendering/gpu-base/src/mid/none_indexed.rs#L3)
- 范围分配器与批次写回：[utility/growable-range-allocator/src/lib.rs:1](../../utility/growable-range-allocator/src/lib.rs#L1)、[webgpu-hook-utils/src/allocator.rs:70](../../platform/graphics/webgpu-hook-utils/src/allocator.rs#L70)
- MIDC 降级（设备与宿主两条路径）：[platform/graphics/webgpu-midc-downgrade:20](../../platform/graphics/webgpu-midc-downgrade/src/lib.rs#L20)
- LOD 转换与级别元数据：[scene/rendering/attribute-mesh-lod/src/lod_convert.rs:1](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L1)
- 网格实体模型与缓冲视图：[scene/core/src/mesh.rs:3](../../scene/core/src/mesh.rs#L3)
