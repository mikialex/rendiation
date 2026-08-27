# Rendiation 场景模型 LOD 指南（scene/rendering/attribute-mesh-lod）

本文梳理 [scene/rendering/attribute-mesh-lod](../../scene/rendering/attribute-mesh-lod/src/lib.rs) 的场景模型 LOD 系统：当属性网格数据变化时，在 CPU 侧用网格简化算法为每个网格生成多级几何（各级索引合并进同一条索引缓冲），渲染时再按"屏幕空间投影误差"逐实体选择画哪一级。LOD 的级别选择与相机投影、节点世界变换（包括视图依赖变换）耦合，并且设备侧（compute 生成间接命令）与宿主侧（host-driven 立即模式）的行为刻意不同。本模块是 attribute mesh 间接渲染（见 [attribute-mesh-indirect-render-guide.md](attribute-mesh-indirect-render-guide.md)）之上的扩展层，不改变基座的池化、寻址与提交结构。

## 前置阅读

| 文档 | 内容 |
| --- | --- |
| [attribute-mesh-indirect-render-guide.md](attribute-mesh-indirect-render-guide.md) | 基座：attribute mesh 顶点/索引池化、`AttributeMeshMeta` 两跳寻址、宿主/设备双通道命令生成（本文大量复用它） |
| [indirect-draw-command-guide.md](indirect-draw-command-guide.md) | mid 层 `DrawCommandBuilder` / `IndirectDrawProvider` 抽象与 MIDC 降级机制（LOD 命令生成器的 trait 载体） |
| [gpu-indirect-batch-collector-guide.md](gpu-indirect-batch-collector-guide.md) | 帧内 provider 创建的组织（`use_create_or_update_indirect_draw_providers` 何时被调用） |
| [skill-translation/shader-edsl-core-zh.md](skill-translation/shader-edsl-core-zh.md) | `Node<T>` 表达式、`loop_by` / `if_by` / `make_local_var`、shader 结构体与 GPU 侧索引加载 |
| [skill-translation/shader-edsl-binding-and-typed-container-zh.md](skill-translation/shader-edsl-binding-and-typed-container-zh.md) | `bind_by` / `bind`、`UniformBufferDataView` / `StorageBufferReadonlyDataView` 等容器 |
| [skill-translation/rendiation-algebra-zh.md](skill-translation/rendiation-algebra-zh.md) | `Mat4` / `Vec` 操作、高精度平移（HPT）与 `into_mat_hpt_storage_pair` 等矩阵分解工具 |
| [skill-translation/scene-core-structure-zh.md](skill-translation/scene-core-structure-zh.md) | 场景模型 / 网格实体、`UriLoadResult` 异步加载、`SceneModelRefNode` 外键 |
| [skill-translation/fundamental-gpu-component-model-zh.md](skill-translation/fundamental-gpu-component-model-zh.md) | `ShaderHashProvider` / `RenderComponent` 组件模型（管线哈希约定） |

## 模式概览

LOD 系统分两级抽象：

- **转换（一次）**：`process_attribute_mesh_lod`（[lod_convert.rs:33](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L33)）在网格数据变化时对每个网格做 CPU 简化，产出若干 `LODLevelInfo`（每级：池内偏移、索引计数、误差）和一条**合并索引缓冲**——原始索引在最前，逐级简化的索引按序拼在后面。合并缓冲是关键设计：所有"不感知 LOD"的消费者（如 rtx 网格取数）看到的仍然是完整网格。
- **选择（逐帧逐实体）**：间接渲染路径中，compute 着色器为每个场景模型生成间接绘制命令时，根据"当前级别误差投影到屏幕后的像素数"与 `lod_error_threshold`（像素阈值）比较，**从粗到细**选第一个投影误差不超过阈值的级别（[draw_cmd.rs:196](../../scene/rendering/attribute-mesh-lod/src/draw_cmd.rs#L196)）。级别选择完全发生在 GPU 上，每帧、每个实体、每个视图独立进行。

几个值得先记住的设计决策：

- **保守估计原则**：误差（级别误差取局部空间最大位移）、世界缩放（取矩阵三列长度的最大值）、距离（取相机到世界包围盒的最近距离）全部向"不低估投影误差"的方向取值，保证画面上的简化误差永远不超过阈值。
- **设备侧与宿主侧行为不同**：设备侧（compute / indirect 路径）做完整级别选择；宿主侧 `draw_command_host_access` **只画根级**（原始网格，即级别 0），当前代码里留有"未来做 LOD 选择"的 todo；GLES 即时模式路径则完全不经过 LOD（直接用原始网格数据）。
- **与视图依赖变换耦合**：级别选择需要的世界缩放来自节点矩阵，而节点矩阵可以被 `view_dependent_transform` 按视图覆盖——因此 occ 样式（视图钉扎）模型的 LOD 决策基于被覆盖后的矩阵，而距离计算仍用场景图的原始世界包围盒。
- **两级元数据表**：`level_meta`（每网格一条：levels 偏移、级别数、根级计数，宿主+设备双通道）与 `lod_levels`（每级别一条：偏移/计数/误差，GPU storage buffer）分离，生命周期由范围分配器管理。

## 核心概念

| 概念 | 定义位置 | 说明 |
| --- | --- | --- |
| `AttributeLODConfig` | [lib.rs:21](../../scene/rendering/attribute-mesh-lod/src/lib.rs#L21) | LOD 转换配置：`lod_conversion_mode`（三种模式）、`min_lod_triangle_count`（少于该三角形数不转换）、`error_double_mode_config` |
| `LODConversionMode` | [lod_convert.rs:91](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L91) | `Disabled` / `HalfCount`（逐级减半）/ `ErrorDoubling`（逐级误差倍增） |
| `ErrorDoublingConfig` | [lod_convert.rs:102](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L102) | 误差倍增模式参数：`base_error_factor`（初始误差 = 包围盒尺寸 × 该因子）与 `max_error_factor` |
| `LODLevelInfo` | [lod_convert.rs:13](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L13) | 单个 LOD 级别：`index_offset`（相对本级网格索引段起点的 u32 槽偏移）、`count`（索引元素数）、`error`（局部空间最大位移误差） |
| `AttributeLODMeshData` | [lod_convert.rs:434](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L434) | 转换结果：`lod_levels`（含根级在内的全部级别）+ `content`（合并索引后的网格内容，顶点不变） |
| `AttributeLODMeshIndirectRenderer` | [lib.rs:121](../../scene/rendering/attribute-mesh-lod/src/lib.rs#L121) | LOD 渲染器：包装内部 `AttributeMeshIndirectRenderer`，新增 level_meta / lod_levels 两张表与相机控制 |
| `AttributeLODMeshIndirectDrawCreator` | [draw_cmd.rs:4](../../scene/rendering/attribute-mesh-lod/src/draw_cmd.rs#L4) | LOD 命令生成器：包装内部 `AttributeMeshIndirectDrawCreator`，宿主侧只画根级，设备侧做级别选择 |
| `AttributeMeshLODIndirectDrawCreatorInvocation` | [draw_cmd.rs:97](../../scene/rendering/attribute-mesh-lod/src/draw_cmd.rs#L97) | 上述 creator 的 compute 侧执行体：`generate_draw_command(draw_id)` 逐 sm 选择级别并生成参数 |
| `LODCameraInfo` | [draw_cmd.rs:16](../../scene/rendering/attribute-mesh-lod/src/draw_cmd.rs#L16) | 级别选择所需相机数据：`camera`（投影+世界位置）、`view_resolution`（视口宽高）、`lod_error_threshold`（像素阈值，只用 x 分量） |
| `CurrentLODCameraControl` | [lib.rs:219](../../scene/rendering/attribute-mesh-lod/src/lib.rs#L219) | 当前活动相机控制：viewer 在每帧每个视口开始时 `set`，结束后 `set(None)` |
| `simplify_by_edge_collapse` | [content/mesh/simplification/src/edge_collapse/mod.rs:28](../../content/mesh/simplification/src/edge_collapse/mod.rs#L28) | QEM 二次误差度量的边折叠简化（meshoptimizer 风格），返回结果索引数与误差 |
| `simplify_sloppy` | [content/mesh/simplification/src/sloppy.rs:12](../../content/mesh/simplification/src/sloppy.rs#L12) | 顶点聚类简化：用网格尺寸二分搜索保证达到计数目标，作为边折叠达不到目标时的回退 |
| `NodeGPUStorageWithOverride` | [extension/view-dependent-transform/src/indirect_draw.rs:206](../../extension/view-dependent-transform/src/indirect_draw.rs#L206) | 视图依赖变换的节点矩阵覆盖：当前视图有覆盖时返回覆盖矩阵，否则走基座 |

## 分层动机与数据流

先看完整数据流，再逐层展开：

```text
viewer_mesh_input (scene/core 异步网格加载, UriLoadResult)
  └─ process_attribute_mesh_lod (lod_convert.rs:33, 项目 rayon 池并行)
       ├─ 可转换检查 → 简化(simplify_by_edge_collapse / simplify_sloppy)
       │     → 合并索引缓冲 → processed_meshes (网格内容, origin 在最前)
       └─ lod_metadata (mesh → Vec<LODLevelInfo>, 每级: 偏移/计数/误差)
  ├─ use_attribute_mesh_indirect_renderer (基座: 顶点池/索引池/AttributeMeshMeta/sm→mesh 映射)
  ├─ use_range_allocated_device_buffers::<LODLevelInfo> (webgpu-hook-utils)
  │    └─ lod_levels 池 (GPU storage buffer) + 每网格分配结果 (levels 偏移, 级别数)
  ├─ level_meta (Vec4<u32> 每 mesh 一条, host+device):
  │    x = levels 偏移, y = 级别数, z = 根级计数(origin 真实索引元素数)
  └─ AttributeLODMeshIndirectRenderer
       ├─ DrawCommandBuilderCreator::make_draw_command_builder (lib.rs:131)
       │    ├─ 索引网格 → AttributeLODMeshIndirectDrawCreator (快照当前 LODCameraInfo)
       │    │    ├─ draw_command_host_access: 只画根级 (host-driven 路径)
       │    │    └─ build_invocation → generate_draw_command (compute):
       │    │         投影误差 = info.error × world_scale × 视口高×focal_y/2 / 距离
       │    │         从粗到细选第一个 ≤ lod_error_threshold 的级
       │    └─ 非索引网格 → 直接透传内部 creator
       └─ IndirectDrawProviderCreator::use_create_or_update_indirect_draw_providers
            └─ use_and_create_default_indirect_draw_provider (mid/mod.rs:84)
                 └─ 逐实体 compute 生成命令 → MultiIndirectDrawCount / MIDCDowngradeBatch
```

分层动机：

- **转换与选择分离**。转换在数据变化时发生一次，可以花大代价（QEM 迭代、并行简化）；选择在渲染时逐帧逐实体发生，必须廉价（一次表查询 + 一次投影计算 + 一趟从粗到细的循环）。
- **合并缓冲保持非 LOD 消费者兼容**。`build_merged_lod_mesh` 保证 origin 索引永远在合并缓冲的最前段，且 `AttributeLODMeshData.content` 是"带更多索引的普通网格"——任何直接消费网格数据的路径（顶点池上传、rtx 网格访问、GLES 即时模式）无需感知 LOD。
- **保守估计原则贯穿三处**：误差取最大位移、缩放取最大轴、距离取最近距离。三者任意一个被低估都会让实际屏幕误差超过阈值（画面出现比预期更粗糙的级）；取保守值则只会偶尔选更细的级（安全方向）。
- **元数据与数据分离**。`lod_levels` 是数据（大，按级别分配），`level_meta` 是索引（小，按 mesh 分配）；前者由范围分配器管理生命周期，后者用宿主备份支持 host 侧读取——与基座 `AttributeMeshMeta` 的模式一致。

## 网格转换：process_attribute_mesh_lod

### 输入输出与并行

`process_attribute_mesh_lod`（[lod_convert.rs:33](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L33)）输入 `UseResult<AttributesMeshDataChangeInput>`（`Arc<LinearBatchChanges<RawEntityHandle, UriLoadResult<AttributesMeshWithVertexRelationInfo>>>`，见 [scene/core/src/mesh.rs:336](../../scene/core/src/mesh.rs#L336)），输出两份变化流（[lod_convert.rs:28](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L28)）：

- `processed_meshes`：与输入同构的网格变化流，加载完成的网格被替换为合并后的内容，未加载的网格原样透传（保证下游能看到状态变化）。
- `lod_metadata`：`LinearBatchChanges<RawEntityHandle, ExternalRefPtr<Vec<LODLevelInfo>>>`，只包含加载完成的网格；未加载的网格不出现在此流中（对应级别分配保持旧值）。

转换在 `map_spawn_stage_in_thread_data_changes` 的 spawn 阶段执行，网格之间互相独立，用 `spawner.install` 投到项目自己的 rayon 池并行（[lod_convert.rs:49](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L49)，而非全局池）。`UriLoadResult` 三态（[utility/abstract-uri-data/src/lib.rs:26](../../utility/abstract-uri-data/src/lib.rs#L26)）：`LivingOrLoaded` 走转换；`PresentButFailedToLoad` / `PresentButNotLoaded` 跳过转换但让网格内容流继续向下游传播状态变化。

### 可转换性检查

`process_lod_attribute_mesh`（[lod_convert.rs:123](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L123)）对每个网格做一连串检查，任何一项不满足就退化为 `only_origin_level`（[lod_convert.rs:443](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L443)，只输出单级：origin，error 为 0，内容原样）：

- 必须**有索引**且**拓扑为 `TriangleList`**（其他拓扑的索引分组语义不适用）。
- 索引字节宽必须是 2（u16）或 4（u32），否则退化。
- `triangle_count < min_lod_triangle_count`（默认 32）退化——简化收益太低（[lod_convert.rs:152](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L152)）。
- 必须有 `Positions` 语义的顶点流，且能按 `Vec3<f32>` 解析；顶点数为 0 退化。
- 索引必须全部落在顶点数范围内（简化只产生对原顶点缓冲的引用，越界索引会导致崩溃，[lod_convert.rs:174](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L174)）。
- 包围盒最大边 `extent <= 1e-6` 退化——简化算法内部把顶点重缩放进单位立方体，零尺寸会破坏缩放（[lod_convert.rs:186](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L186)）。

注意简化过程**只引用原顶点缓冲**（不生成新顶点），所以合并后的网格内容顶点部分原样保留（[lod_convert.rs:425](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L425)），只有索引被替换。

### 两种简化模式

`LODConversionMode`（[lod_convert.rs:91](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L91)）决定级别如何生成：

**HalfCount（默认）**：`simplify_half_count`（[lod_convert.rs:210](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L210)）每轮把目标索引数减半，重复直到目标低于 `min_lod_triangle_count * 3`。每轮先跑 `simplify_by_edge_collapse`（QEM 边折叠），`EdgeCollapseConfig` 里 `target_error = f32::MAX`、`use_absolute_error = true`、`lock_border = true`——**只受计数约束，不受误差约束**（[lod_convert.rs:233](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L233)）。若拓扑太复杂导致结果索引数仍大于目标（`result_count > target`），回退到 `simplify_sloppy` 保证达到计数目标（[lod_convert.rs:245](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L245)）——sloppy 的 `target_error` 传 `extent`，注释说明不能超过 extent，否则其内部网格尺寸计算会坏（[lod_convert.rs:247](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L247)，`min_grid = 1/target_error` 会退到 0 触发断言）。

**ErrorDoubling**：`simplify_error_doubling`（[lod_convert.rs:283](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L283)）初始目标误差 `extent * base_error_factor`（默认 0.001，即包围盒的千分之一），每轮翻倍直到 `max_error_factor`（默认 0.1）或计数下限。每轮边折叠的 `target_index_count` 直接给最小计数、`target_error` 给当前目标误差——**误差限制主导简化**（[lod_convert.rs:310](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L310)）。这是 viewer 默认配置文件选用的模式（`viewer_init_config.toml`，见下文「用户视角」）。

两种模式共同的收尾逻辑：

- 每轮结束后若结果计数没有减少（`result_count >= prev_count`）或低于计数下限，停止（拓扑约束卡死）。
- **误差强制单调递增**：`error = result_error.max(prev_error)`（[lod_convert.rs:270](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L270)）。GPU 选择算法依赖"级别越粗误差越大"这一性质（见「设备侧 LOD 选择」）；sloppy 的误差度量与边折叠不可比，因此显式取历史最大值兜底。

`LODConversionMode::Disabled` 直接 `only_origin_level`——网格内容原样、元数据单级，设备侧 `has_lod` 判定（级别数 > 1）不成立，行为与基座一致，即"禁用转换也禁用 LOD 效果"（[lod_convert.rs:96](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L96)）。

### 简化算法：error 的语义

`simplify_by_edge_collapse`（[edge_collapse/mod.rs:28](../../content/mesh/simplification/src/edge_collapse/mod.rs#L28)）是 QEM（二次误差度量）迭代边折叠：

- 顶点先重缩放进单位立方体（`rescale_positions`，[lib.rs:44](../../content/mesh/simplification/src/lib.rs#L44)），`vertex_scale = extent`。
- 每个顶点维护 quadric（由面、顶点、边贡献累加），每轮按 quadric 误差排序候选折叠边，逐个执行并锁定邻接顶点，`has_triangle_flips` 防止三角形翻转（[edge_collapse/mod.rs:425](../../content/mesh/simplification/src/edge_collapse/mod.rs#L425)）。
- `use_absolute_error = true` 时，`error_limit = target_error² / extent²` 把绝对误差换算进单位立方体空间（[edge_collapse/mod.rs:93](../../content/mesh/simplification/src/edge_collapse/mod.rs#L93)）；返回的 `result_error = sqrt(最大 quadric 误差) × extent`（[edge_collapse/mod.rs:157](../../content/mesh/simplification/src/edge_collapse/mod.rs#L157)）。

因此 `result_error` 是"折叠过程中产生的最大顶点位移"，还原到**网格局部空间的绝对单位**——这正是 `LODLevelInfo.error` 的语义（"mesh 的 local space 中网格与简化网格的最大距离"，[lod_convert.rs:24](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L24)）。`simplify_sloppy`（[sloppy.rs:12](../../content/mesh/simplification/src/sloppy.rs#L12)）走顶点聚类：按网格尺寸做二分+插值搜索，找到三角形数不超过目标的网格密度，每个网格单元内用 quadric 选代表顶点，误差取各单元 quadric 误差的平方根最大值再乘 `error_scale`（[sloppy.rs:136](../../content/mesh/simplification/src/sloppy.rs#L136)）。

### 合并索引缓冲：build_merged_lod_mesh

`build_merged_lod_mesh`（[lod_convert.rs:343](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L343)）把 origin 与各级简化索引拼成一条缓冲，布局为：

```text
[origin_index, coarser_level_index, coarser_level_index, ...]
```

级别 0 固定为 origin：`index_offset = 0`、`count = origin_indices.len()`、`error = 0`（[lod_convert.rs:350](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L350)）。两个格式细节值得注意：

- **不做 u16 → u32 的转换**（反之亦然）：绘制派发器按 origin 网格的索引类型决定索引缓冲格式，合并缓冲必须保持同一类型（[lod_convert.rs:357](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L357)）。
- **u16 时每级尾部补齐到偶数元素**（[lod_convert.rs:365](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L365)）：u16 两个元素占一个 u32 槽。`LODLevelInfo.index_offset` 的单位是 **u32 槽**（与 `AttributeMeshMeta.index_offset` 相同，[lod_convert.rs:19](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L19)），如果某级起点落在奇数元素位置，设备端 base index 将无法表达跨级边界（原生 MIDC 以 u16 元素为单位、降级模式以 u32 槽为单位，见下文）。补齐的哨兵元素永远不会被画到——每级 `count` 是真实元素数，绘制计数不会包含它。u32 索引天然对齐 u32 槽，无需补齐。

## level_meta / 多级网格的组织

`use_attribute_lod_mesh_indirect_renderer`（[lib.rs:42](../../scene/rendering/attribute-mesh-lod/src/lib.rs#L42)）是装配入口，先跑 `process_attribute_mesh_lod` 得到两份流，再调用基座 `use_attribute_mesh_indirect_renderer` 用处理后的网格建立池与元数据表，最后建立 LOD 自己的两张表：

**`lod_levels`（数据表）**：`use_range_allocated_device_buffers::<LODLevelInfo>`（[lib.rs:68](../../scene/rendering/attribute-mesh-lod/src/lib.rs#L68)，hook-utils 的范围分配器封装，[webgpu-hook-utils/src/lib.rs:48](../../platform/graphics/webgpu-hook-utils/src/lib.rs#L48)）。每个网格的级别数组在池里占一段，段内顺序即"从细到粗"。分配器在 spawn 阶段做增删与重定位（`apply_resize` 处理增长），CreateRender 阶段把级别数据写进 GPU 缓冲（`RangeAllocateBufferUpdates::write`，[allocator.rs:211](../../platform/graphics/webgpu-hook-utils/src/allocator.rs#L211)）。分配结果（每网格 `[u32; 2]`：段起点 + 段长，见 [allocator.rs:79](../../platform/graphics/webgpu-hook-utils/src/allocator.rs#L79) 的 `Value = [u32; 2]`）是 `level_meta` 的来源。

**`level_meta`（索引表）**：`use_storage_buffer_with_host_backup::<Vec4<u32>>`（[lib.rs:80](../../scene/rendering/attribute-mesh-lod/src/lib.rs#L80)），以 **mesh 分配索引** 为下标（`use_max_item_count_by_db_entity::<AttributesMeshEntity>`，[lib.rs:104](../../scene/rendering/attribute-mesh-lod/src/lib.rs#L104)）。每个元素的三分量：

- `x` = 该网格级别段在 `lod_levels` 的起点（由分配结果在字段偏移 0 处写入，[lib.rs:85](../../scene/rendering/attribute-mesh-lod/src/lib.rs#L85)）；
- `y` = 级别数（分配结果的第二项）；
- `z` = **根级计数**：origin 的**真实索引元素数**（由 `root_count_changes` 在 `offset_of!(Vec4<u32>, z)` 处写入，[lib.rs:87](../../scene/rendering/attribute-mesh-lod/src/lib.rs#L87)，取 `levels.first()` 即 origin 级）。

`z` 是宿主侧"只画根级"时用的计数，语义是**真实索引元素数**（u16 时以 u16 元素计，不含补齐哨兵）——与基座宿主侧画命令里「展开后」的计数一致：基座 `AttributeMeshMeta.count` 字段本身以 u32 槽计（u16 时每槽两个索引），基座 host 访问在取数时把 `count` 展开成真实元素数（`×2`，补零尾槽再 `-1`，见 [attribute-mesh-indirect-render-guide.md](attribute-mesh-indirect-render-guide.md) 的「宿主侧画命令」）；LOD 的 `z` 直接就是展开后的值，所以 LOD 的 host 访问可以用它直接作为计数、不必再做展开。

这张表同时有 GPU 副本与宿主备份：GPU 副本供 compute 侧按 mesh id 读（`level_meta.index(mesh_handle).load()`），宿主备份供 host-driven 路径按 `alloc_index()` 读（`level_meta_host.get(mesh_id.alloc_index())`）——与基座 `AttributeMeshMeta` 的双通道模式相同。

## 设备侧 LOD 选择

`generate_draw_command`（[draw_cmd.rs:110](../../scene/rendering/attribute-mesh-lod/src/draw_cmd.rs#L110)）在 compute 中执行。前半段（两跳寻址、u16 展开、fallback）与基座同构，区别在于：

- 从 `level_meta` 读出 `(level_start, level_count, root_count)`（[draw_cmd.rs:121](../../scene/rendering/attribute-mesh-lod/src/draw_cmd.rs#L121)）。
- **fallback 计数**（[draw_cmd.rs:133](../../scene/rendering/attribute-mesh-lod/src/draw_cmd.rs#L133)）：`has_level_meta`（`level_count > 0` 且 `level_start != DEVICE_RANGE_ALLOCATE_FAIL_MARKER`）为真时用 `root_count`（只画根级），否则退回整网格计数（`meta.count` 做 u16 展开）——后者覆盖"网格数据仍加载中、尚无级别元数据"的场景。

### 投影误差公式

级别选择的核心是把 `LODLevelInfo.error`（局部空间位移）投影成屏幕像素（[draw_cmd.rs:143](../../scene/rendering/attribute-mesh-lod/src/draw_cmd.rs#L143)）：

```text
投影误差(px) = world_error × viewport_height × focal_y / (2 × distance)   [透视]
投影误差(px) = world_error × viewport_height × focal_y / 2                 [正交]
```

- `world_error = info.error × world_scale`：局部误差乘世界缩放（见下）。
- `viewport_height`：视口像素高（`view_resolution.y`，[draw_cmd.rs:149](../../scene/rendering/attribute-mesh-lod/src/draw_cmd.rs#L149)）。
- `focal_y = camera_projection.y().y()`（[draw_cmd.rs:152](../../scene/rendering/attribute-mesh-lod/src/draw_cmd.rs#L152)）：标准透视矩阵的 `(1,1)` 项是 `1/tan(fov/2)`，正交矩阵是 `2/frustum_height`，两者都直接充当"像素/单位"的缩放因子。
- 透视与正交通过 `camera_projection.z().w()` 区分（[draw_cmd.rs:157](../../scene/rendering/attribute-mesh-lod/src/draw_cmd.rs#L157)）：标准透视矩阵 z 列的 w 分量是 -1，正交矩阵是 0。
- **保守距离**：透视时取相机位置到世界包围盒的最近距离——把相机位置 `clamp` 进包围盒（AABB 上的最近点），再算距离，最小截断到 `1e-6` 防除零（[draw_cmd.rs:160](../../scene/rendering/attribute-mesh-lod/src/draw_cmd.rs#L160)）。距离取保守最小值 ⟹ 投影误差取保守最大值 ⟹ 不会画出超过阈值的粗糙度。相机位置只用高精度平移的 f32 低半部分（`f1`），注释说明 LOD 选择对距离精度不敏感（[draw_cmd.rs:165](../../scene/rendering/attribute-mesh-lod/src/draw_cmd.rs#L165)）。

### 世界缩放

`sm_node_info.get_node_info_value(draw_id)` 取该实体的 `NodeStorage`，`world_scale` 取 `world_matrix_none_translation`（去掉平移的世界矩阵）三列向量长度的**最大值**（[draw_cmd.rs:174](../../scene/rendering/attribute-mesh-lod/src/draw_cmd.rs#L174)）——列向量长度即该轴的世界缩放，取最大轴保证非均匀缩放下误差不被低估。

这里就是与视图依赖变换耦合的入口：`sm_node_info` 是 `IndirectNodeInfoSceneModelAccess` trait 对象，viewer 装配时传入的可能是被 `NodeGPUStorageWithOverride` 包装的访问器（见「与 view_dependent_transform 的耦合」），返回的矩阵可能不是场景图矩阵。

### 从粗到细的选择循环

```rust
let has_lod = level_count > 1 && level_start != FAIL_MARKER;
if_by(has_lod, || {
  let level_index = (level_count - 1).make_local_var();   // 从最粗级开始
  loop_by(|cx| {
    let info = lod_levels.index(level_start + level_index.load()).load().expand();
    let projected_error = info.error * world_scale * pixel_per_unit;
    let should_select =
      projected_error.less_equal_than(error_threshold).or(level_index.load().equals(val(0)));
    if_by(should_select, || {
      selected_offset.store(info.index_offset);
      selected_count.store(info.count);
      cx.do_break();
    });
    level_index.store(level_index.load() - val(1));        // 向更细级走
  });
});
```

（[draw_cmd.rs:184-215](../../scene/rendering/attribute-mesh-lod/src/draw_cmd.rs#L184)）

判据与收敛性质：

- **从粗到细**：级别数组按"origin（最细）、…、最粗"存放，`level_count - 1` 是最粗级。误差单调递增（转换阶段强制），所以"第一个投影误差 ≤ 阈值的级别"就是满足质量要求的最粗级别——画得最省。
- **级别 0 兜底**：`should_select` 在 `level_index == 0` 时恒真（`error = 0` 恒小于阈值），循环必然在级别 0 终止；`selected_offset` / `selected_count` 的初值也是 fallback（根级或整网格），`has_lod` 不成立时直接采用初值。
- 投影误差与阈值都是 `f32` 像素量纲，比较用 `less_equal_than`（等于阈值也通过）。

### 命令组装与 MIDC 降级

选中级别后（[draw_cmd.rs:217](../../scene/rendering/attribute-mesh-lod/src/draw_cmd.rs#L217)）：

- `base_index = meta.index_offset + selected_offset`：池内网格段起点（u32 槽）+ 级内偏移（u32 槽），两者单位一致可直接相加（这正是每级 u32 槽对齐的意义，[lod_convert.rs:369](../../scene/rendering/attribute-mesh-lod/src/lod_convert.rs#L369)）。
- 非降级且 u16 时 `base_index × 2`（换算成 u16 元素单位）；降级模式下索引池以 u32 槽在设备端读取，直接使用（与基座一致）。
- 命令的 `vertex_count = selected_count`（该级真实索引元素数）、`instance_count = 1`、`base_instance = draw_id`。

所有 LOD 级别都在同一条合并索引缓冲里，级别切换只是换 `base_index` 与 `vertex_count`，顶点池与索引缓冲绑定完全不变——这也是合并缓冲设计带来的零成本切换。

## host 侧行为：只画根级

`draw_command_host_access`（[draw_cmd.rs:25](../../scene/rendering/attribute-mesh-lod/src/draw_cmd.rs#L25)）是宿主侧命令生成：

- 读内部 `vertex_address_buffer_host` 得到池内段起点；`index_offset == DEVICE_RANGE_ALLOCATE_FAIL_MARKER` 返回 `None`（数据未就绪，跳过该实体）。
- 计数直接取 `level_meta_host` 的 `z`（根级计数），`indices: start..start+count`，注释明确"host access 只画根级（origin mesh），不做 LOD 选择"并留了 todo（[draw_cmd.rs:27](../../scene/rendering/attribute-mesh-lod/src/draw_cmd.rs#L27)）。

这条路径由 host-driven 模式消费：`process_host_driven_indirect_draws`（[host_driven.rs:3](../../scene/rendering/gpu-indirect/src/host_driven.rs#L3)）按 shader key 分类后，对每个实体的 `draw_command_host_access` 生成 `DrawIndexedIndirectArgsStorage`，再经 `downgrade_multi_indirect_draw_count_host_driven` 打包成单段 indirect + helper。host-driven 模式下**所有网格都画根级**，LOD 不生效（设备侧的级别选择需要 GPU 上的相机 uniform 与逐实体计算，host 路径没有这套环境）。

GLES 即时模式路径（`use_attribute_mesh_renderer`，[gpu-gles/src/shape/attribute.rs:7](../../scene/rendering/gpu-gles/src/shape/attribute.rs#L7)）则完全不经过 LOD：它从**未经转换**的网格变化流直接建顶点/索引 buffer 并逐实体绑定绘制（frame_all.rs 的 Gles 分支里 `mesh_changes` 没有经过 `process_attribute_mesh_lod`，[frame_all.rs:172](../../application/viewer-content/src/rendering/frame_all.rs#L172)）。

另外注意**阴影 pass 也做 LOD**：`LightSystem::prepare` 在渲染阴影图时同样设置 `LODCameraInfo`（阴影相机 + 阴影图尺寸 + 同一阈值，[lighting/mod.rs:111](../../application/viewer-content/src/rendering/lighting/mod.rs#L111)），因此阴影几何与主视图使用相同的 LOD 规则。

## 与 view_dependent_transform 的耦合

viewer 中间接路径的节点装配顺序（[frame_all.rs:251-285](../../application/viewer-content/src/rendering/frame_all.rs#L251)）：

```rust
let node = use_node_storage(cx);                                  // 基座: 场景图节点矩阵
let node = use_view_dependent_transform_indirect_gpu(             // 视图依赖覆盖包装
  cx, view_camera_source, node, active_view_control.clone());
let mesh = use_attribute_lod_mesh_indirect_renderer(cx, …, node.clone(), …);
```

`use_view_dependent_transform_indirect_gpu`（[view-dependent-transform/src/indirect_draw.rs:3](../../extension/view-dependent-transform/src/indirect_draw.rs#L3)）把节点渲染器包装成 `OverrideNodeIndirectGPU`：视图依赖变换数据（`ViewSceneModelKey = (ViewKey, sm)` → 覆盖矩阵，来自 [occ.rs:16](../../extension/view-dependent-transform/src/occ.rs#L16) 的 occ 样式配置，即"三面体/屏幕角标/正面相机"等视图钉扎装饰模型）按视图存进 GPU（`PerViewGPUResource`：`index_remap` + 稀疏更新矩阵池）。其 `make_component_indirect` 产出 `NodeGPUStorageWithOverride`（[indirect_draw.rs:206](../../extension/view-dependent-transform/src/indirect_draw.rs#L206)），实现同一个 `IndirectNodeInfoSceneModelAccess` trait：

- `get_node_info` 先查当前视图的覆盖表（`index_remap[sm]`），命中则返回覆盖矩阵，否则回落到基座场景图矩阵（[indirect_draw.rs:244](../../extension/view-dependent-transform/src/indirect_draw.rs#L244)）。

LOD creator 的设备侧调用 `sm_node_info.get_node_info_value(draw_id)`（[draw_cmd.rs:175](../../scene/rendering/attribute-mesh-lod/src/draw_cmd.rs#L175)）拿到的就是这份"可能被覆盖"的 `NodeStorage`，因此耦合点有两处：

- **世界缩放**来自覆盖矩阵的列长最大值——occ 样式模型（例如恒定屏幕尺寸的 `Screen2d` / `Triedron` 模式）的 LOD 决策基于其视图依赖缩放，而不是场景图变换。
- **保守距离**仍使用 `sm_world_aabb_info`（`DrawUnitWorldBoundingProvider`，由 viewer culling 提供，基于场景图变换的世界包围盒，[culling.rs:43](../../application/viewer-content/src/rendering/culling.rs#L43)，存储侧实现见 [world_bounding.rs:24](../../scene/rendering/gpu-base/src/world_bounding.rs#L24)）——覆盖矩阵不参与包围盒计算，所以距离与缩放可以来自不同的变换来源。

管线哈希同样耦合：`NodeGPUStorageWithOverride::hash_pipeline` 把"当前视图是否有覆盖"哈希进 PSO key（[indirect_draw.rs:218](../../extension/view-dependent-transform/src/indirect_draw.rs#L218)），LOD creator 的 `ShaderHashProvider` 又把 node 访问器与包围盒访问器的类型哈希并入（[draw_cmd.rs:88](../../scene/rendering/attribute-mesh-lod/src/draw_cmd.rs#L88)），因此 LOD 计算着色器与顶点阶段共享同一套"是否含视图依赖覆盖"的缓存键。

## trait 抽象体系与接入

`AttributeLODMeshIndirectRenderer`（[lib.rs:121](../../scene/rendering/attribute-mesh-lod/src/lib.rs#L121)）对基座 `AttributeMeshIndirectRenderer` 是"**包装式扩展**"，三个 trait 的实现全部走同样的组合模式：

- **`DrawCommandBuilderCreator`**（[lib.rs:131](../../scene/rendering/attribute-mesh-lod/src/lib.rs#L131)）：先取内部 creator 与是否索引（`internal.make_draw_command_builder_impl`）；**索引网格**包成 `AttributeLODMeshIndirectDrawCreator`，**非索引网格直接透传**内部 creator（非索引没有级别可切，`NoneIndexedDrawCommandBuilder` 分支见 [lib.rs:150](../../scene/rendering/attribute-mesh-lod/src/lib.rs#L150)）。
- **`IndirectDrawProviderCreator`**（[lib.rs:157](../../scene/rendering/attribute-mesh-lod/src/lib.rs#L157)）：`get_impl_distinguish_key_by_impl_select_id` 用 `TypeId + 是否索引` 区分实现（索引类型不影响实现差异，与基座约定一致）；`use_create_or_update_indirect_draw_providers` 把 builder 交给 mid 层的 `use_and_create_default_indirect_draw_provider`（[gpu-base/src/mid/mod.rs:84](../../scene/rendering/gpu-base/src/mid/mod.rs#L84)），并透传 `used_in_midc_downgrade`。
- **`IndirectModelShapeRenderImpl`**（[lib.rs:190](../../scene/rendering/attribute-mesh-lod/src/lib.rs#L190)）：形状组件、索引池暴露、shader key 全部委托内部。

`AttributeLODMeshIndirectDrawCreator` 本身实现 `IndexedDrawCommandBuilder`（host 访问 + `build_invocation` + `bind`）与 `ShaderHashProvider`，是 [indirect-draw-command-guide.md](indirect-draw-command-guide.md)「模板二：包装已有 builder」的典型实例：`bind` 时先绑自己的 7 份资源（元数据、level_meta、sm→mesh、lod_levels、相机三件套）再透传 node / aabb 访问器绑定（[draw_cmd.rs:75](../../scene/rendering/attribute-mesh-lod/src/draw_cmd.rs#L75)）；管线哈希合并内部哈希与 node / aabb 类型哈希（[draw_cmd.rs:88](../../scene/rendering/attribute-mesh-lod/src/draw_cmd.rs#L88)）。

另一个值得注意的接入细节：**`make_draw_command_builder` 会快照当前 LOD 相机**——`self.current_lod_camera.get().expect("active_lod_camera not set")`（[lib.rs:143](../../scene/rendering/attribute-mesh-lod/src/lib.rs#L143)）。这意味着该渲染器的 provider 创建必须发生在"当前视图相机已设置"的作用域内：viewer 在每视口 `render()` 前 `lod_camera_control.set(Some(...))`、渲染结束后 `set(None)`（[frame_all.rs:645-659](../../application/viewer-content/src/rendering/frame_all.rs#L645)），阴影 pass 同理（[lighting/mod.rs:111](../../application/viewer-content/src/rendering/lighting/mod.rs#L111)）。

## 用户视角：配置与调用

### 转换配置

`AttributeLODConfig`（[lib.rs:21](../../scene/rendering/attribute-mesh-lod/src/lib.rs#L21)）是 serde 配置，viewer 的默认配置见 [viewer_init_config.toml:51](../../viewer_init_config.toml#L51)：

```toml
[init_only.indirect_attribute_mesh_lod_config]
lod_conversion_mode = "ErrorDoubling"   # Disabled / HalfCount / ErrorDoubling
min_lod_triangle_count = 32

[init_only.indirect_attribute_mesh_lod_config.error_double_mode_config]
base_error_factor = 0.001
max_error_factor = 0.1
```

运行时默认值在 `AttributeLODConfig::default`（`HalfCount`，[lib.rs:32](../../scene/rendering/attribute-mesh-lod/src/lib.rs#L32)），代码路径见 [init_config.rs:45](../../application/viewer-content/src/init_config.rs#L45)（`indirect_attribute_mesh_lod_config` 字段）。

### 误差阈值

`attribute_mesh_lod_threshold_pixels`（默认 2.0 像素，[init_config.rs:183](../../application/viewer-content/src/init_config.rs#L183)）是 `LODCameraInfo.lod_error_threshold` 的唯一来源（[frame_all.rs:653](../../application/viewer-content/src/rendering/frame_all.rs#L653)），运行时可用 egui 滑条在 1.0..4.0 之间调整（[egui.rs:57](../../application/viewer-content/src/rendering/egui.rs#L57)）。语义：级别切换的判据，级别投影到屏幕后的误差超过它就会切到更细的级；值越小画得越精细。

### 装配调用

```rust
let (cx, lod_camera_control) = cx.use_plain_state_default::<CurrentLODCameraControl>();
// ...
let mesh = use_attribute_lod_mesh_indirect_renderer(
  cx,
  &init_config.indirect_attribute_mesh_init,        // 基座池容量配置
  &init_config.indirect_attribute_mesh_lod_config,  // LOD 转换配置
  init_config.using_texture_as_storage_buffer_for_indirect_rendering, // merge_with_vertex_allocator
  self.using_host_driven_indirect_draw,             // force_midc_downgrade
  mesh_changes,
  node.clone(),                                     // 可能被 view_dependent_transform 包装
  culling.as_ref().map(|v| v.bounding_provider.clone()),
  lod_camera_control.clone(),
);
```

（[frame_all.rs:271](../../application/viewer-content/src/rendering/frame_all.rs#L271)）。返回的 `AttributeLODMeshIndirectRenderer` 与 cell_mesh 并列进 `Vec<Box<dyn IndirectModelShapeRenderImpl>>`，经 `use_viewer_std_model_renderer` 与材质合并成模型实现，再进 `use_indirect_scene_model`（[frame_all.rs:377](../../application/viewer-content/src/rendering/frame_all.rs#L377)）参与帧内批提取与 provider 创建（组织链路见 [gpu-indirect-batch-collector-guide.md](gpu-indirect-batch-collector-guide.md)）。

## 使用要点

- **error 单位是网格局部空间**：`LODLevelInfo.error` 由简化器以"顶点位移"返回（乘以包围盒 extent 还原绝对单位），shader 里必须乘世界缩放才得到世界误差。`max_error_factor = 0.1` 意味着最粗级别允许包围盒尺寸 10% 的位移。
- **level 0 即 origin**：合并缓冲的 origin 段永远在起点、错误为 0，任何"不感知 LOD"的消费者读到的就是完整网格；`level_meta.z`（根级计数）是真实索引元素数，等价于基座宿主侧画命令「展开后」的计数（基座 `AttributeMeshMeta.count` 本身以 u32 槽计），host 侧无需再做 u16 展开。
- **u16 对齐是硬约束**：`index_offset` 以 u32 槽为单位，u16 索引的每级（包括 origin 段）尾部补齐到偶数元素，否则原生/降级两种模式的 base index 都会错位。补齐元素不会被画到。
- **阈值是像素**：`lod_error_threshold` 只有 x 分量有效；设备侧与阈值比较的是"保守投影误差"，永远不小于真实屏幕误差。
- **级别选择只在 indirect 路径生效**：device（compute 间接命令）逐实体选级；host-driven（`draw_command_host_access`）只画根级；GLES 即时模式完全无 LOD；阴影 pass 与主视图共享同一相机控制与阈值。
- **`active_lod_camera not set` 会 panic**：LOD 渲染器要求 provider 创建时当前相机已设置，自建渲染管线的用户需要在自己的视口/阴影作用域内 `CurrentLODCameraControl::set`，与 `CurrentViewControl` 的模式一致。

## 延伸阅读

- 基座实现（池化、元数据、双通道命令）：[scene/rendering/gpu-indirect/src/shape/attribute/mod.rs:42](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L42)、对应 guide [attribute-mesh-indirect-render-guide.md](attribute-mesh-indirect-render-guide.md)
- mid 层 builder / provider / 降级：[scene/rendering/gpu-base/src/mid/mod.rs:84](../../scene/rendering/gpu-base/src/mid/mod.rs#L84)、[indirect-draw-command-guide.md](indirect-draw-command-guide.md)
- 网格简化算法：[content/mesh/simplification/src/edge_collapse/mod.rs:28](../../content/mesh/simplification/src/edge_collapse/mod.rs#L28)、[content/mesh/simplification/src/sloppy.rs:12](../../content/mesh/simplification/src/sloppy.rs#L12)
- 范围分配器与宿主备份缓冲：[platform/graphics/webgpu-hook-utils/src/lib.rs:48](../../platform/graphics/webgpu-hook-utils/src/lib.rs#L48)、[webgpu-hook-utils/src/hook.rs:161](../../platform/graphics/webgpu-hook-utils/src/hook.rs#L161)
- 节点矩阵与视图依赖覆盖：[scene/rendering/gpu-indirect/src/node.rs:103](../../scene/rendering/gpu-indirect/src/node.rs#L103)、[extension/view-dependent-transform/src/indirect_draw.rs:3](../../extension/view-dependent-transform/src/indirect_draw.rs#L3)、[extension/view-dependent-transform/src/occ.rs:16](../../extension/view-dependent-transform/src/occ.rs#L16)
- 世界包围盒提供者：[scene/rendering/gpu-base/src/world_bounding.rs:3](../../scene/rendering/gpu-base/src/world_bounding.rs#L3)
- host-driven 命令生成：[scene/rendering/gpu-indirect/src/host_driven.rs:3](../../scene/rendering/gpu-indirect/src/host_driven.rs#L3)
