# Rendiation extension 间接绘制实现模式指南（wide-line / wide-styled-points / text-3d / cell-mesh）

本文梳理 [extension](../../extension) 目录下四个「扩展几何类型」的完整实现套路：宽线（wide-line）、宽点（wide-styled-points）、三维文字（text-3d）与单元网格（cell-mesh）。它们都不是标准 `AttributesMeshEntity` 网格，而是各自带着独立数据模型、独立几何生成与独立拾取逻辑的「渲染扩展」。四个扩展共享同一套接入架构，可以总结为 **builder + key + picker 三件套**：

- **builder**：间接绘制命令的生成器。扩展如何把自己「每实体一份」的几何数据变成 GPU 能按子列表一次画完的 `DrawIndirectArgsStorage`。
- **key**：增量 PSO 分组键。扩展实体如何进入 `SceneModelGroupKey` 的 `ForeignHash` 槽位，从而与标准网格、彼此之间天然分桶。
- **picker**：CPU 拾取实现。扩展如何在 geometry-query 的局部层实现射线/视锥求交，与渲染链路完全解耦。

除此之外还有两个配套件：**数据模型注册**（`register_*_data_model`，定义实体/组件/外键）与**局部包围盒共享计算**（`*_SceneModelLocalBounding`，供拾取预筛与动态 BVH 使用）。

## 前置阅读

四个扩展都挂在 scene model 的 payload 外键上，走增量 PSO key 分桶、间接命令生成与 CPU 几何拾取三条既有链路，建议先了解：

| 文档 | 内容 |
| --- | --- |
| [skill-translation/scene-core-structure-zh.md](skill-translation/scene-core-structure-zh.md) | SceneModelEntity / StandardModelEntity、payload 外键、节点与可见性 |
| [skill-translation/database-schema-zh.md](skill-translation/database-schema-zh.md) | 实体/组件/外键声明、反向引用视图 |
| [skill-translation/query-system-zh.md](skill-translation/query-system-zh.md) | DualQuery 增量模型、fanout 扇出、select/union 组合子 |
| [skill-translation/fundamental-gpu-component-model-zh.md](skill-translation/fundamental-gpu-component-model-zh.md) | ShaderHashProvider / ShaderPassBuilder / GraphicsShaderProvider 与管线哈希 |
| [skill-translation/shader-edsl-binding-and-typed-container-zh.md](skill-translation/shader-edsl-binding-and-typed-container-zh.md) | StorageBufferDataView、bind_by 与 pass 侧绑定 |
| [skill-translation/shader-edsl-core-zh.md](skill-translation/shader-edsl-core-zh.md) | 着色器结构体、内存布局（std430）、控制流 |
| [skill-translation/shader-edsl-graphics-zh.md](skill-translation/shader-edsl-graphics-zh.md) | 顶点/片元阶段、内置语义、混合与深度状态 |
| [batch-extractor-guide.md](batch-extractor-guide.md) | 增量 PSO key 与 `GroupKeyForeignImpl` 扩展机制（本指南的 key 部分直接建立其上） |
| [gpu-indirect-batch-collector-guide.md](gpu-indirect-batch-collector-guide.md) | 批 → 实现分类 → provider → 渲染的帧内链路（builder 部分的消费端） |
| [indirect-draw-command-guide.md](indirect-draw-command-guide.md) | mid 层 `IndirectDrawProvider` / `DrawCommandBuilder` 与 MIDC 降级 |
| [geometry-query-guide.md](geometry-query-guide.md) | `LocalModelPicker` 分层与拾取容差（picker 部分建立其上） |

## 模式概览

四个扩展放在一起看，实现骨架高度一致。以 wide-line 为例先给整体图景：

```text
数据模型（lib.rs）
  WideLineModelEntity + 组件（宽度/颜色/样式/缓冲区…）
  SceneModelWideLineRenderPayload：SceneModelEntity → WideLineModelEntity 外键

key 层（indirect_draw.rs 或 lib.rs）
  use_wide_line_group_key：组件值 → SceneModelGroupKey::ForeignHash{TypeId + 状态, alpha}
    └─ 进入 GroupKeyForeignImpl.model 槽 → 与标准模型天然分桶

渲染器（indirect_draw.rs）
  use_widen_line_indirect_renderer（两阶段 hook）
    三块资源：几何池（use_range_allocated_device_buffers，顶点数据 → 范围分配器 → GPU 池）
             参数行（use_storage_buffer_with_host_backup，每扩展实体一行：范围 + 组件值）
             索引映射（use_db_device_foreign_key，sm id → 扩展实体 id 的 u32 表）
    三个 trait：builder（NoneIndexedDrawCommandBuilder，host 现场算 + compute 生成 双表达）
               key 实现（get_impl_distinguish_key_by_impl_select_id，实现分类）
               渲染（IndirectModelRenderImpl，shape 组件绑定三段 buffer + 顶点展开几何）

拾取（pick.rs）
  WideLinePicker：LocalModelPicker + AbstractMesh 视图
  WideLineSceneModelLocalBounding：SharedResultProvider 局部包围盒
```

其余三个扩展只是在这个骨架里换「几何数据内容」与「顶点展开方式」，接入点（外键槽位、trait 实现、viewer 装配位置）一一对应。下表先给全貌，后面逐层展开：

| 扩展 | 实体 / payload 外键 | key 槽位 | 渲染器类型 | 拾取几何 | 局部包围盒 |
| --- | --- | --- | --- | --- | --- |
| wide-line | `WideLineModelEntity` / `SceneModelWideLineRenderPayload`（SceneModelEntity 上） | `GroupKeyForeignImpl.model` | `IndirectModelRenderImpl`（独立渲染器） | 线段，容差 = 线宽/2（ScreenSpace） | `WideLineSceneModelLocalBounding` |
| wide-styled-points | `WideStyledPointsEntity` / `SceneModelWideStyledPointsRenderPayload`（SceneModelEntity 上） | `GroupKeyForeignImpl.model` | `IndirectModelRenderImpl`（独立渲染器） | 每点按像素宽在 NDC 膨胀成两个三角形 | `WideStyledPointsSceneModelLocalBounding` |
| text-3d | `Text3dEntity` / `SceneModelText3dPayload`（SceneModelEntity 上） | `GroupKeyForeignImpl.model` | `IndirectModelRenderImpl`（独立渲染器） | 逐字 hit_box 两个三角形 | `Text3dSceneModelLocalBounding` |
| cell-mesh | `CellMeshEntity` / `StandardModelCellMeshPayload`（**StandardModelEntity** 上） | `GroupKeyForeignImpl.mesh` | `IndirectModelShapeRenderImpl`（**std model 的 shape 槽**） | 每单元两个三角形（按 shrink_ratio 收缩） | `use_cell_mesh_local_bounding` |

一个关键分岔在表里已经可见：**前三个是「独立的 scene model 类型」**（在 `GroupKeyForeignImpl.model` 槽接入，拥有自己的 `IndirectModelRenderImpl` 渲染器），**cell-mesh 是「标准模型的一种形状」**（在 `GroupKeyForeignImpl.mesh` 槽接入，作为 `IndirectModelShapeRenderImpl` 混入 `SceneStdModelIndirectRenderer` 的 shape 列表，与属性网格平级）。后者的材质、蒙皮、状态覆盖全部复用标准模型路径，这是它与前三个最大的架构差异，详见「cell-mesh：作为 std model 形状的扩展」一节。

## 数据模型注册：payload 外键与组件

每个扩展都在自己的 `lib.rs` 里提供 `register_*_data_model(sparse: bool)`，声明实体表、组件与「SceneModelEntity（或 StandardModelEntity）→ 扩展实体」的 payload 外键。以 wide-line 为例（[extension/wide-line/src/lib.rs:25](../../extension/wide-line/src/lib.rs#L25)）：

```rust
pub fn register_wide_line_data_model(sparse: bool) {
  global_entity_of::<SceneModelEntity>()
    .declare_sparse_foreign_key_maybe_sparse::<SceneModelWideLineRenderPayload>(sparse);
  global_database()
    .declare_entity::<WideLineModelEntity>()
    .declare_component::<WideLineWidth>()
    // ... 其余组件
    .declare_component::<WideLineMeshBuffer>();
}
```

- `declare_foreign_key!`（[wide-line/src/lib.rs:42](../../extension/wide-line/src/lib.rs#L42)）建立 `SceneModelEntity → WideLineModelEntity` 的反向外键。scene model 一侧只需写 `SceneModelWideLineRenderPayload` 一个组件即完成挂载，节点归属（`SceneModelRefNode`）、场景归属（`SceneModelBelongsToScene`）与标准模型完全相同——扩展模型与标准模型在场景图层面没有区别。
- 几何数据用 `ExternalRefPtr<Vec<T>>` 组件持有（宿主侧共享指针，如 `WideLineMeshBuffer`，[lib.rs:64](../../extension/wide-line/src/lib.rs#L64)），顶点结构体用 `#[repr(C)] + ShaderVertex + Facet` 声明，语义标记（`#[semantic(WideLinePosition)]`）同时服务序列化与 GLES 顶点绑定。
- viewer 在启动时统一注册全部扩展的数据模型（[application/viewer-content/src/lib.rs:133](../../application/viewer-content/src/lib.rs#L133)），gui-3d 还单独注册 wide-line（[extension/gui-3d/src/lib.rs:81](../../extension/gui-3d/src/lib.rs#L81)）。

## 渲染器骨架：几何池 + 参数行 + 索引映射

四个扩展的 `use_*_indirect_renderer`（宽点叫 `use_widen_styled_points_indirect_renderer`、cell-mesh 叫 `use_cell_mesh_renderer`）都是同一个两阶段 hook 模式（详见 [query-hook-guide.md](query-hook-guide.md) 与 [hooks-guide.md](hooks-guide.md)）：spawn 阶段做宿主侧数据映射与分配器更新，render 阶段（`CreateRender`）把数据写进 GPU，最后 `cx.when_render` 产出渲染器结构体。三块核心资源：

### 几何池：use_range_allocated_device_buffers

数据上传用 webgpu-hook-utils 的通用池化工具（[platform/graphics/webgpu-hook-utils/src/lib.rs:48](../../platform/graphics/webgpu-hook-utils/src/lib.rs#L48)）：

```rust
let data_source = cx.use_dual_query::<WideLineMeshBuffer>()
  .map_spawn_stage_in_thread_dual_query(cx, move |source_info| {
    source_info.delta().into_change().collective_map(|buffer| {
      // 把数据库里的 [WideLineVertex] 逐字段映射成 GPU 布局 [WideLineVertexStorage]
      ExternalRefPtr::new(new_buffer)
    })
  });
let (segments, allocation_info) = use_range_allocated_device_buffers::<WideLineVertexStorage>(
  cx, "wide line segment buffer pool", 100, u32::MAX, data_source);
```

- `data_source` 是「扩展实体 id → 顶点列表」的增量数据源，spawn 阶段在宿主侧把数据库组件映射为 GPU 内存布局的结构体数组。
- `use_range_allocated_device_buffers` 内部持有一个 `GrowableRangeAllocator`（与 batch-extractor 的 id 池同源，见 [utility/growable-range-allocator](../../utility/growable-range-allocator/src/lib.rs)）：每个扩展实体在池里分得一段 `(offset, count)`，数据变更只做局部写入；分配失败时产出 `[DEVICE_RANGE_ALLOCATE_FAIL_MARKER, 0]`（[allocator.rs:70](../../platform/graphics/webgpu-hook-utils/src/allocator.rs#L70)），因此 GPU 侧可以放心用「count 为 0」当作「未分配」。
- 返回的 `allocation_info.allocation_changes` 会同步进参数行（见下），让顶点池的范围与参数行保持一致。
- 分配出的 `AbstractReadonlyStorageBuffer<[T]>` 直接进渲染器结构体，顶点阶段用 `bind_by` 绑定后按偏移索引。

### 参数行：use_storage_buffer_with_host_backup

每个扩展实体「一行」的参数（数据范围 + 各组件值）镜像到 GPU：

```rust
let (cx, params) = cx.use_storage_buffer_with_host_backup::<WideLineParameters>(
  "wide line buffer parameters and range info", 128, u32::MAX);
let offset = std::mem::offset_of!(WideLineParameters, data_range);
range_change.update_storage_array_with_host(cx, params, offset);
let change = cx.use_dual_query::<WideLineWidth>().into_delta_change();
change.update_storage_array_with_host(cx, params, /* offset_of!(width) */);
// ... 其余组件字段
params.use_max_item_count_by_db_entity::<WideLineModelEntity>(cx);
params.use_update(cx);
```

- `use_storage_buffer_with_host_backup` 与 `update_storage_array_with_host` 的「with_host」保证宿主侧也有同一份数据（`params.buffer.make_read_holder()`），供 host-driven 路径现场生成绘制命令（见 builder 一节）。
- `use_max_item_count_by_db_entity` 让缓冲容量跟随数据库实体数增长，是「宿主表 → GPU 表」的通用镜像惯例。
- `WideLineParameters` 的 GPU 布局（[indirect_draw.rs:146](../../extension/wide-line/src/indirect_draw.rs#L146)）：`data_range: Vec2<u32>`（池内范围）+ 全部影响绘制的组件值。宽点加了 `color_alpha_texture: TextureSamplerHandlePair`，由 `use_tex_watcher_with_host` 维护纹理句柄（[wide-styled-points/src/indirect_draw.rs:68](../../extension/wide-styled-points/src/indirect_draw.rs#L68)）；text-3d 的 `TextMeta` 则把 `local_matrix`（`Text3dLocalTransform`）也放进了参数行（[text-3d/src/indirect_draw.rs:61](../../extension/text-3d/src/indirect_draw.rs#L61)）。

### 索引映射：use_db_device_foreign_key

```rust
let sm_to_wide_line_device = use_db_device_foreign_key::<SceneModelWideLineRenderPayload>(cx);
```

（[webgpu-hook-utils/src/lib.rs:30](../../platform/graphics/webgpu-hook-utils/src/lib.rs#L30)）为每个 `SceneModelEntity` 分配一个 u32 槽位，写入「它的扩展实体 id 或 u32::MAX」。这是整条链路的两跳间接寻址（与标准模型的 `sm_to_std_model_device` 同构，见 [gpu-indirect-batch-collector-guide.md](gpu-indirect-batch-collector-guide.md)）：

```text
顶点阶段 LogicalRenderEntityId（当前实例的 sm id）
  → sm_to_xxx_device[sm_id]        （u32 表，一跳）
  → params[xxx_id]                 （参数行，二跳：范围 + 组件值）
  → 几何池[range.x + i]            （三跳：顶点数据）
```

## builder：间接绘制命令的双表达

「每实体一份几何、一个子列表一次画完」是间接绘制的核心诉求。四个扩展的绘制命令都是 **NoneIndexed**（顶点数由数据量决定、无索引缓冲，`get_index_storage_buffer` 统一返回 `Some(None)`），顶点展开全部在顶点着色器里完成。

### DrawCommandBuilderCreator：按实体取 builder

`IndirectDrawProviderCreator` / `DrawCommandBuilderCreator` 两个 trait 是实现的分类与命令入口（定义在 [gpu-indirect/src/scene.rs:114](../../scene/rendering/gpu-indirect/src/scene.rs#L114)、[:127](../../scene/rendering/gpu-indirect/src/scene.rs#L127)）。四个扩展的模式完全一致：

```rust
impl DrawCommandBuilderCreator for WideLineModelIndirectRenderer {
  fn make_draw_command_builder(&self, id: RawEntityHandle) -> Option<DrawCommandBuilder> {
    let id = unsafe { EntityHandle::from_raw(id) };
    let line = self.model_access.get(id)?;            // 不是我的实体就 None，让下一个实现试
    let creator = WideLineDrawCreator { params: ..., sm_to_wide_line_device: ..., ... };
    DrawCommandBuilder::NoneIndexed(Box::new(creator)).into()
  }
}

impl IndirectDrawProviderCreator for WideLineModelIndirectRenderer {
  fn get_impl_distinguish_key_by_impl_select_id(&self, id: RawEntityHandle) -> Option<u64> {
    let line = self.model_access.get(id)?;
    fast_hash_scope(|hasher| {
      self.type_id().hash(hasher);
      use_native_line.hash(hasher);   // wide-line 特有：native 1px 优化是独立的实现
    }).into()
  }

  fn use_create_or_update_indirect_draw_providers(&self, cx, list, dispatch_info, id)
    -> Option<Vec<Box<dyn IndirectDrawProvider>>> {
    let cmd_builder = self.make_draw_command_builder(id)?;
    use_and_create_default_indirect_draw_provider(
      list, dispatch_info_device_offset_compacted, cmd_builder, cx, self.used_in_midc_downgrade,
    ).into()
  }
}
```

- `get_impl_distinguish_key_by_impl_select_id` 的返回值是「实现分类 key」：帧内 `use_make_scene_batch_pass_content` 按它对子列表做二次分组（不同扩展各画各的）。宽线是唯一在其中再哈希一个运行期条件（`use_native_line`）的实现——native 1px 线与宽线网格的顶点展开完全不同，不能共用一个绘制调用。
- `use_create_or_update_indirect_draw_providers` 四份代码逐字相同：取 builder → 交给 mid 层的 `use_and_create_default_indirect_draw_provider`（[gpu-base/src/mid/mod.rs:84](../../scene/rendering/gpu-base/src/mid/mod.rs#L84)），由它完成「compute 逐实体生成命令 → 按子列表切片 provider → MIDC 降级」的通用工作（细节见 [indirect-draw-command-guide.md](indirect-draw-command-guide.md)）。扩展只需提供「怎么算一个实体的命令」，不需要碰命令池。

### NoneIndexedDrawCommandBuilder：host 与 compute 的双表达

`WideLineDrawCreator`（[wide-line/src/indirect_draw.rs:464](../../extension/wide-line/src/indirect_draw.rs#L464)）是 builder 模式的完整范本，其余三个的 DrawCreator 与之同构：

```rust
impl NoneIndexedDrawCommandBuilder for WideLineDrawCreator {
  fn draw_command_host_access(&self, id) -> Option<DrawCommand> {
    // host 路径：读宿主侧参数行，直接算出一个 DrawCommand
    let model = self.sm_to_wide.get(id).unwrap();
    let param = self.params_host.get(model.alloc_index()).unwrap();
    let seg_count = /* 按 is_line_strip 从 data_range.y 换算 */;
    DrawCommand::Array { instances: 0..1, vertices: 0..stride * seg_count }.into()
  }

  fn build_invocation(&self, cx: &mut ShaderComputePipelineBuilder)
    -> Box<dyn NoneIndexedDrawCommandBuilderInvocation> {
    // compute 路径：把 GPU buffer 绑定进命令生成 shader
    let params = cx.bind_by(&self.params);
    let sm_to_wide_line_device = cx.bind_by(&self.sm_to_wide_line_device);
    Box::new(DrawCmdBuilderInvocation { params, sm_to_wide_line_device, ... })
  }
}
```

`DrawCmdBuilderInvocation::generate_draw_command`（compute 侧，[wide-line/src/indirect_draw.rs:529](../../extension/wide-line/src/indirect_draw.rs#L529)）对每个实体（`draw_id` = 池内 sm 下标）算出一条命令：

```rust
fn generate_draw_command(&self, draw_id: Node<u32>) -> Node<DrawIndirectArgsStorage> {
  let line_id = self.sm_to_wide_line_device.index(draw_id).load();  // 一跳
  let vertex_count = self.params.index(line_id).data_range().load().y(); // 二跳：数据量
  let seg_count = is_line_strip.select(vertex_count.max(1) - 1, vertex_count / 2);
  let stride = if self.use_native_line { 2 } else { 18 };
  ENode::<DrawIndirectArgsStorage> {
    vertex_count: val(stride) * seg_count,
    instance_count: val(1),
    base_vertex: val(0),
    base_instance: draw_id,   // 关键：base_instance 写回 sm id
  }.construct()
}
```

要点：

- **vertex_count 由数据量决定，stride 是每几何单元的固定顶点数**。宽线每段 18 顶点（或 native 2）、宽点每点 6、文字每字 6、cell-mesh 每单元 6。命令生成只需要读参数行的 `data_range.y`，不需要知道几何内容。
- **`base_instance = draw_id` 是实体 id 的传递通道**。渲染阶段顶点着色器从 `VertexIndex / stride` 得到实例下标，而实例下标只在「该实体的几何池切片内」有效；实体的全局身份靠 `LogicalRenderEntityId`（由 provider 的 invocation source 注册，见 [gpu-base/src/mid/mod.rs:38](../../scene/rendering/gpu-base/src/mid/mod.rs#L38)）经 `sm_to_xxx_device` 解析——`base_instance` 本身只是占位，实际用不到。
- **host 与 compute 双表达的一致性**：`draw_command_host_access` 读宿主侧 `params_host`（`SparseStorageBufferWithHostRaw` 的 host 副本），`generate_draw_command` 读 GPU 侧同名字段，两者的换算公式逐行对应。GL 后端（无 GPU 流压缩能力）走 host 表达，indirect 后端走 compute 表达，见 [gpu-indirect-batch-collector-guide.md](gpu-indirect-batch-collector-guide.md) 的 host-driven 分支。
- 分配失败时 `data_range.x == DEVICE_RANGE_ALLOCATE_FAIL_MARKER`：host 表达显式返回 `None`（该实体不画），compute 表达依赖「失败时 count 为 0」自然画空（各文件注释都强调这一约定）。

## key：增量 PSO 分组键

### ForeignHash 与 TypeId

key 层的接入点由 [batch-extractor-guide.md](batch-extractor-guide.md) 的 `GroupKeyForeignImpl` 机制提供：`model` / `material` / `mesh` 三个槽位各接受一个可选的 key 查询，用 `dual_query_select` 覆盖默认 key。四个扩展的实现都是「组件值 → `ForeignHash`」的同一套路，以宽线为例（[extension/wide-line/src/indirect_draw.rs:10](../../extension/wide-line/src/indirect_draw.rs#L10)）：

```rust
pub fn use_wide_line_group_key(cx, use_native_line_for_one_width_line: bool)
  -> UseResult<BoxedDynDualQuery<RawEntityHandle, SceneModelGroupKey>> {
  let sm_ref_wide_line = cx.use_db_rev_ref_tri_view::<SceneModelWideLineRenderPayload>();
  cx.use_dual_query::<WideLineDepthEnable>()
    .dual_query_zip(cx.use_dual_query::<WideLineTransparent>())
    .dual_query_zip(cx.use_dual_query::<WideLineWidth>())
    .fanout(sm_ref_wide_line, cx)   // 组件 key 是扩展实体，扇出到 SceneModelEntity
    .dual_query_map(move |((enable_depth, trans), width)| SceneModelGroupKey::ForeignHash {
      internal: fast_hash_scope(|hasher| {
        std::any::TypeId::of::<WideLineModelEntity>().hash(hasher);   // 类型身份，永不撞桶
        (enable_depth, trans).hash(hasher);
        if use_native_line_for_one_width_line {
          (width == 1.0).hash(hasher);   // init-only 配置，注释强调不可变
        }
      }),
      require_alpha_blend: trans,
    })
    .dual_query_boxed()
}
```

共同点与差异：

| 扩展 | key 查询 | hash 内容 | require_alpha_blend |
| --- | --- | --- | --- |
| wide-line | `use_wide_line_group_key`（[indirect_draw.rs:10](../../extension/wide-line/src/indirect_draw.rs#L10)） | TypeId + depth + transparent +（native 优化时）width==1.0 | `trans` 组件透传 |
| wide-points | `use_wide_styled_points_group_key`（[indirect_draw.rs:9](../../extension/wide-styled-points/src/indirect_draw.rs#L9)） | TypeId + depth_test_enabled | 恒 true |
| text-3d | `use_text3d_group_key`（[indirect_draw.rs:9](../../extension/text-3d/src/indirect_draw.rs#L9)） | 仅 TypeId | 恒 true |
| cell-mesh | `use_cell_mesh_group_key`（[indirect_draw.rs:15](../../extension/cell-mesh/src/indirect_draw.rs#L15)，`MeshGroupKey::ForeignHash`，mesh 槽） | 仅 TypeId | 由标准材质 key 决定 |

- **TypeId 保证扩展类型永不与标准模型或其他扩展撞桶**，这是 `ForeignHash` 的第一性要求。
- 进 hash 的组件只挑「影响 PSO 的」：深度开关、混合（alpha）、native 线宽特判。颜色、样式、文字内容等「只影响数据不影响管线」的值绝不进 key——它们变化时只需重写参数行/几何池，不换管线。
- 宽点的 `require_alpha_blend: true` 是硬编码而非组件透传——宽点永远 alpha 混合（片元里有透明度/纹理 alpha），透明过滤（`SceneContentKey`）因此始终把它们送进透明批。
- 注意 `use_native_line_for_one_width_line` 与 `use_wide_line_vertices_count` 的同类参数一样，是 **init-only 配置**：注释明确「must be immutable for every call / in all time」。它进了 key 哈希但不在任何增量查询里，运行期改它会导致 key 与 PSO 不一致。

### key 与 PSO 哈希的一致性约定

间接渲染要求「子列表内共享一条管线」，而子列表由 `SceneModelGroupKey` 分桶、管线由 `hash_shader_group_key` 决定，两套哈希必须一致。wide-line 的对照最清晰：

- 提取器 key（host 侧，增量维护）：`TypeId::of::<WideLineModelEntity>() + depth + trans (+width==1.0)`。
- PSO 哈希 `hash_shader_group_key`（[indirect_draw.rs:216](../../extension/wide-line/src/indirect_draw.rs#L216)）：`depth + use_native_line + transparent`，再经 `hash_shader_group_key_with_self_type_info`（[gpu-indirect/src/std_model.rs:11](../../scene/rendering/gpu-indirect/src/std_model.rs#L11)）补 `hasher.hash(self.as_any().type_id())`——**类型身份在两边都以 hash 形式出现**，只是 host 侧用实体 `TypeId`、PSO 侧用渲染器 `TypeId`，互相对应。
- 实现分类 key（`get_impl_distinguish_key_by_impl_select_id`）：`type_id + use_native_line`——保证 native 与非 native 的宽线分到不同绘制调用。

三类哈希同源、内容互洽，是扩展接入时的硬约束；破坏它会表现为「同子列表不同 PSO」（渲染错误）或「同 PSO 分多桶」（性能退化），`classify_draws` 里 `hash_shader_group_key_with_self_type_info` 失败即过滤实体（[gpu-indirect/src/scene.rs:16](../../scene/rendering/gpu-indirect/src/scene.rs#L16)）。

## 渲染组件：顶点展开与片元

`shape_renderable_indirect` 返回的组件绑定三段 buffer（索引映射表、参数行、几何池），并实现 `GraphicsShaderProvider` 做顶点展开。四个扩展的绑定结构一致（以 wide-points 为例，[indirect_draw.rs:233](../../extension/wide-styled-points/src/indirect_draw.rs#L233)）：

```rust
impl ShaderPassBuilder for WidePointsIndirectDrawComponent<'_> {
  fn setup_pass(&self, ctx: &mut GPURenderPassCtx) {
    ctx.binding.bind(self.sm_to_wide_points_device);
    ctx.binding.bind(self.params);
    ctx.binding.bind(self.points);
  }
}
```

顶点展开的公共骨架（见「顶点展开差异」一节，各有几何内容差异）：

```text
VertexIndex
  → instance_index = VertexIndex / stride；vertex_in_unit = VertexIndex % stride
  → 扩展实体 id = sm_to_xxx_device[LogicalRenderEntityId]
  → 参数行：data_range（池内起点）、组件值
  → 几何池[data_range.x + instance_index] → 展开成 N 个顶点（switch_by(vertex_in_unit)）
  → 屏幕空间扩张（宽线/宽点按像素宽偏移 ClipPosition；文字/cell-mesh 直接画）
```

片元侧共同点：都 `insert_type_tag::<UnlitMaterialTag>()`（拒绝光照）；透明都在 `frag_output` 上开 `ALPHA_BLENDING`；深度可配置（宽线 `depth_compare = Always + 不写深度` 当 `!enabled_depth`；宽点 `depth_write_enabled = false` + 按 `rev_z` 选 NearerEqual；文字不写深度）。

`model_info_injector` 与 `material_renderable_indirect` 四个扩展都返回 `Some(Box::new(()))`——它们没有 id 注入逻辑、没有材质。cell-mesh 走的是 std model 路径，其 id 注入由 `SceneStdModelIdInjector` 完成，材质由标准材质组件完成。

## picker：CPU 拾取与包围盒

拾取体系见 [geometry-query-guide.md](geometry-query-guide.md)：每个扩展实现 `LocalModelPicker`（局部空间 primitive 查询）与一个 `AbstractMesh` 视图（把数据库数据变成 primitive 迭代器），世界层（矩阵、预筛、容差换算）由 `SceneModelPickerBaseImpl` 免费提供。四个 picker 的注册点都在 viewer 的局部 picker 链里（[application/viewer-content/src/pick.rs:159](../../application/viewer-content/src/pick.rs#L159)）：

```rust
let local_model_pickers: Vec<Box<dyn LocalModelPicker>> = vec![
  Box::new(attribute_mesh_picker),
  Box::new(wide_line_picker),
  Box::new(wide_point_picker),
  Box::new(text_picker),
  Box::new(cell_mesh_picker),
];
```

链条按顺序尝试，每个 picker 只认自己的 payload 外键，查不到返回 `None` 让下一个试。

### 容差机制

点/线类几何需要「拾取容差」否则射线永远打不中，机制由 `bounding_enlarge_tolerance` 报告、`SceneModelPickerBaseImpl` 换算（[geometry-query-guide.md](geometry-query-guide.md) 的 `compute_local_tolerance`）：

| picker | 容差 | 换算类型 | 含义 |
| --- | --- | --- | --- |
| `WideLinePicker` | `IntersectTolerance::new(line_width / 2., ScreenSpace)`（[pick.rs:59](../../extension/wide-line/src/pick.rs#L59)） | ScreenSpace | 线宽一半（屏幕像素），远近恒定手感 |
| `WidePointsPicker` | 各点宽度最大值（共享查询，[pick.rs:71](../../extension/wide-styled-points/src/pick.rs#L71)） | ScreenSpace | 最大点宽 |
| `TextPicker` / `CellMeshPicker` | `Some(None)`（不需要容差） | - | 三角形几何直接求交 |

### 各自的 AbstractMesh 视图

- **wide-line**：`WideLinePickView`（[pick.rs:156](../../extension/wide-line/src/pick.rs#L156)）把顶点对解释为 `LineSegment`——`is_line_strip` 时相邻顶点成段（`primitive_index` 与 `primitive_index + 1`），否则每对顶点成段（`2i, 2i+1`）。射线与线段求交的参数就是容差。
- **wide-points**：`WidePointTriMeshView`（[pick.rs:164](../../extension/wide-styled-points/src/pick.rs#L164)）是**唯一需要世界矩阵做屏幕空间膨胀的局部实现**：每个点经 `camera_ctx.camera_vp` 投影到 NDC，按 `(width + extra_tolerance) * view_size_inv` 膨胀成正方形、切成两个三角形，再投影回局部空间求交。`primitive_index / 2` 还原点下标。广角下这个正方形在局部空间不是真实的，但投影回屏幕恰好是目标像素块——这正是 `LocalModelPicker` 请求里带 `world_mat` 与 `camera_ctx` 的原因（详见 [geometry-query-guide.md](geometry-query-guide.md) 对 WidePointsPicker 的说明）。
- **text-3d**：`TextPickView`（[text-3d/src/pick.rs:94](../../extension/text-3d/src/pick.rs#L94)）用 `SlugBuffer.hit_boxes`（逐字包围盒，数据准备阶段由 cosmic_text 排版产出）切成两个三角形，再乘 `Text3dLocalTransform`——拾取不碰字形轮廓曲线，只按包围盒，命中即「点中这个字」（`primitive_index / 2` = 字下标）。
- **cell-mesh**：`CellMeshPickView`（[cell-mesh/src/pick.rs:62](../../extension/cell-mesh/src/pick.rs#L62)）把每单元四个角点按 `shrink_ratio` 向 center 收缩后切成两个三角形——拾取几何与渲染几何逐像素一致。

### 局部包围盒：SharedResultProvider

拾取预筛、frustum 查询与动态 BVH 都需要「sm → 局部包围盒」的共享查询。四个扩展各自提供 `SharedResultProvider`（或等价函数），viewer 侧在 `SceneModelLocalBounding`（[application/viewer-content/src/bounding.rs:3](../../application/viewer-content/src/bounding.rs#L3)）里用 `dual_query_select` 合并：

```rust
let wide_line_sm_bounding = cx.use_shared_dual_query(WideLineSceneModelLocalBounding);
let wide_point_sm_bounding = cx.use_shared_dual_query(WideStyledPointsSceneModelLocalBounding);
let text3d_sm_bounding = cx.use_shared_dual_query(Text3dSceneModelLocalBounding(self.0.clone()));
let cell_mesh_bounding = use_cell_mesh_local_bounding(cx);
let extra = wide_line_sm_bounding.dual_query_select(wide_point_sm_bounding)
  .dual_query_boxed().dual_query_select(text3d_sm_bounding)
  .dual_query_boxed().dual_query_select(cell_mesh_bounding).dual_query_boxed();
```

实现模式（以 wide-line 为例，[pick.rs:6](../../extension/wide-line/src/pick.rs#L6)）：对组件数据 `use_dual_query_execute_map` 现算 `Box3<f32>`（宽点取所有点包围盒、text-3d 先算 `SlugBuffer` 再乘局部矩阵、cell-mesh 五个点含 center 全收），再沿 payload 外键 fanout 到 `SceneModelEntity`。text-3d 的包围盒依赖字形数据，因此 `Text3dSceneModelLocalBounding` 直接复用 `Text3dSlugBuffer` 共享计算（[data_prepare.rs:49](../../extension/text-3d/src/data_prepare.rs#L49)）。cell-mesh 的外键是 `StandardModelCellMeshPayload`，需要**两次 fanout**（`CellMeshEntity → StandardModelEntity → SceneModelEntity`，[cell-mesh/src/pick.rs:8](../../extension/cell-mesh/src/pick.rs#L8)）。

## 顶点展开差异：四个扩展的几何生成

### wide-line：18 顶点 sprite 与 native 优化

宽线的几何核心是 [draw.rs:10](../../extension/wide-line/src/draw.rs#L10) 的 `wide_line_vertex`：线段的两个端点在 NDC 里确定方向 `dir`，取垂直偏移 `offset`，按线宽缩放到屏幕像素；顶点的 (x, y) 模板坐标控制三种位移——`x < 0` 翻转偏移方向、`y > 1` 沿 `dir` 延伸（端帽）、`y < 0` 翻转（另一端帽）。

顶点展开（[indirect_draw.rs:329](../../extension/wide-line/src/indirect_draw.rs#L329)）里每段生成 18 个顶点 = 3 行 × 6 顶点（三角形列表），行落在 y ∈ [1,2]、[-1,1]、[-2,-1]（`dy = vertex_index / 6` 与模板算术），中间行双倍宽度覆盖线段、上下两行构成端帽；`discard_by_round_corner`（[draw.rs:89](../../extension/wide-line/src/draw.rs#L89)）在 |uv.y| > 1 的帽区按单位圆剔除，得到圆头。线型（虚线）效果由 `discard_by_line_pattern`（[draw.rs:100](../../extension/wide-line/src/draw.rs#L100)）在片元里按屏幕坐标 + `style_pattern` / `style_factor` 逐段取模剔除。

**native 1px 优化**是 wide-line 独有：`use_native_line_for_one_width_line && width == 1.0` 时走 `PrimitiveTopology::LineList`，stride 降为 2，直接画 CPU 顶点对——不需要展开几何，1px 线获得原生线宽。该开关影响 key、实现分类 key、PSO 哈希、绘制命令四个地方，是「init-only 不可变」约束最严格的字段。

GLES 路径（[gles_draw.rs:249](../../extension/wide-line/src/gles_draw.rs#L249)）不走间接展开：`expand_wide_line_segments` 在宿主侧把顶点对展开成 `WideLineSegmentInstance` 实例数据，配合固定 18 索引的模板 quad（[gles_draw.rs:276](../../extension/wide-line/src/gles_draw.rs#L276)）实例化绘制。

### wide-styled-points：6 顶点/点与 SDF 点样式

每点 6 顶点（1 quad），`switch_by(vertex_index % 6)` 展开（[indirect_draw.rs:254](../../extension/wide-styled-points/src/indirect_draw.rs#L254)），再经与宽线共用的屏幕空间扩张函数（[point_style.rs:3](../../extension/wide-styled-points/src/point_style.rs#L3)，`wide_line_vertex`，注意它与宽线的同名函数是两个独立实现）按 `width` 像素膨胀。

**点样式**是它的独有特性：每个顶点带 `style_id`，片元里 `point_style_entry`（[point_style.rs:41](../../extension/wide-styled-points/src/point_style.rs#L41)）按 id 切换到 16 种 SDF 样式（实心、十字、星形、圆环、球形渐变等），每种样式是一个「片元坐标 → 距离场」函数，配合 `smoothstep` 反锯齿；样式 ≥ 16 时 alpha 恒 1（不裁剪）。颜色 = 参数行 `color` × 顶点 `per_point_color_alpha` × 纹理 `color_alpha_texture`（经 `GPUTextureBindingSystem::indirect_sample` 采样，[indirect_draw.rs:331](../../extension/wide-styled-points/src/indirect_draw.rs#L331)）。

### text-3d：slug 曲线渲染与三池结构

text-3d 的数据管线最深。宿主侧 [data_prepare.rs](../../extension/text-3d/src/data_prepare.rs) 用 cosmic_text 排版：`create_slug_buffer_from_text3d_content`（[:190](../../extension/text-3d/src/data_prepare.rs#L190)）产出 `SlugBuffer`（逐字 hit_box + 定位字形 + 字形集合），字形轮廓经 `extract_curves` 转为二次贝塞尔曲线、按 8×8 band 组织成 `SlugGlyph`（曲线预排序，供片元按像素所在 band 快速遍历）。`FontSystem` 持有一个进程级字形缓存（`slug_glyph_cache`），相同 `CacheKey` 的字形只解析一次。

间接路径 [indirect_data_prepare.rs](../../extension/text-3d/src/indirect_data_prepare.rs) 把 `SlugBuffer` 拍平为**三个 GPU 池**：`curves`（所有曲线的贝塞尔控制点）、`bands`（每字形的 band 头 + band 内曲线索引）、`vertices`（每字形一个 quad 的 `TextGlyphQuad`：对象空间/em 空间范围、band 变换与索引）。`TextMeta` 参数行把三个池的范围、局部矩阵与颜色按实体记下（[indirect_draw.rs:164](../../extension/text-3d/src/indirect_draw.rs#L164)）。

渲染时（[indirect_draw.rs:300](../../extension/text-3d/src/indirect_draw.rs#L300)）：顶点阶段每字 6 顶点展开 quad，位置 = `Text3dLocalTransform * obj_space_pos`；片元阶段 `IndirectSlugShaderDataSource`（[:419](../../extension/text-3d/src/indirect_draw.rs#L419)）按 `band_transform` 定位当前像素所在 band，从 bands 池取该 band 的曲线列表，逐曲线做二次贝塞尔距离场求覆盖度（`slug_shader.rs` 的 `SlugShaderComputer`），覆盖度乘颜色输出。同一份 `FontSystem` + `SlugBuffer` 在 GLES 路径被重排成「字形图集纹理 + 顶点/索引缓冲」（[gles_data_prepare.rs](../../extension/text-3d/src/gles_data_prepare.rs)）。

### cell-mesh：作为 std model 形状的扩展

cell-mesh 的接入与前三者不同：它挂在 **StandardModelEntity** 的 payload 外键上（[cell-mesh/src/lib.rs:44](../../extension/cell-mesh/src/lib.rs#L44)），key 进 `GroupKeyForeignImpl.mesh` 槽（`MeshGroupKey::ForeignHash`），渲染器实现 `IndirectModelShapeRenderImpl`（[shape/mod.rs:11](../../scene/rendering/gpu-indirect/src/shape/mod.rs#L11)）而非 `IndirectModelRenderImpl`。viewer 装配时它被混进 std model 的 shape 列表（[frame_all.rs:309](../../application/viewer-content/src/rendering/frame_all.rs#L309)）：

```rust
let mesh = cx.when_render(|| {
  Box::new(vec![
    Box::new(mesh.unwrap()) as Box<dyn IndirectModelShapeRenderImpl>,  // 属性网格
    cell_mesh.unwrap(),                                                 // 单元网格
  ]) as Box<dyn IndirectModelShapeRenderImpl>
});
```

于是材质（PbrMR/PbrSG/Unlit/occ）、蒙皮、状态覆盖、id 注入全部走 `SceneStdModelIndirectRenderer` 的既有路径（[std_model.rs:342](../../scene/rendering/gpu-indirect/src/std_model.rs#L342)），cell-mesh 只提供形状组件 `CellMeshShape`（[indirect_draw.rs:211](../../extension/cell-mesh/src/indirect_draw.rs#L211)）与命令生成。PSO 哈希经 `IndirectModelShapeRenderImpl::hash_shader_group_key_with_self_type_info` 自动带上 cell-mesh 的类型身份，与属性网格天然分桶（[shape/mod.rs:34](../../scene/rendering/gpu-indirect/src/shape/mod.rs#L34)）。

几何：`CellMeshUnitData` 每单元存 p1..p4 四个角点 + 收缩中心 + 前后双面颜色（三角形以退化四边表示，[cell-mesh/src/lib.rs:20](../../extension/cell-mesh/src/lib.rs#L20)）。顶点阶段每单元 6 顶点按 `switch_by` 展开成两个三角形，位置 = `shrink_ratio.mix(center, corner)`（[indirect_draw.rs:236](../../extension/cell-mesh/src/indirect_draw.rs#L236)）。`CellMeshDisplayMode2D` 组件已声明但渲染器未消费（当前只影响数据模型层）。注意：**cell-mesh 没有 GLES 实现**，GLES 后端下 cell-mesh 模型不可见（见「常见疑问」）。

## 与整条链路的衔接

### batch-extractor：key 层

viewer 的间接分支（[frame_all.rs:387](../../application/viewer-content/src/rendering/frame_all.rs#L387)）把四个扩展的 key 组装进 `GroupKeyForeignImpl`：

```rust
let wide_line_key = use_wide_line_group_key(cx, use_native_line_for_one_width_line);
let wide_point_key = use_wide_styled_points_group_key(cx);
let text_key = use_text3d_group_key(cx);
let impl_key = wide_line_key.dual_query_select(wide_point_key).dual_query_boxed()
  .dual_query_select(text_key).dual_query_boxed();        // model 槽：三个独立扩展互斥
let occ_material = rendiation_occ_style_material::indirect::use_occ_material_indirect_group_key(cx);
let cell_mesh = use_cell_mesh_group_key(cx);              // mesh 槽
let key_impl = GroupKeyForeignImpl { model: Some(impl_key), material: Some(occ_material), mesh: Some(cell_mesh) };
let internal = use_scene_model_group_key(cx, key_impl, mesh_key);
// ... use_scene_model_group_key_with_scene_id_and_visible_filter → occ layer → extractor
```

`dual_query_select` 的互斥约束（两侧 key 集合不重叠）由「不同扩展实体不同 payload 外键」天然保证。提取器（`use_occ_incremental_device_scene_batch_extractor`）对 `SceneModelGroupKey` 完全泛型，扩展实体与标准实体共用同一个 id 池、按 key 分桶成子列表——`impl_select_ids`（每组第一个实体）即代表实体，帧内分类时用它挑实现（[gpu-indirect-batch-collector-guide.md](gpu-indirect-batch-collector-guide.md) 的二次分类）。

### gpu-indirect：实现层与渲染层

`IndirectModelRenderImpl`（[gpu-indirect/src/std_model.rs:5](../../scene/rendering/gpu-indirect/src/std_model.rs#L5)）是「完整渲染实现」的抽象：PSO 哈希、id 注入、形状组件、索引缓冲、材质组件。`Vec<Box<dyn IndirectModelRenderImpl>>` 与 `Box<dyn IndirectModelRenderImpl>` 的 blanket impl（[:49](../../scene/rendering/gpu-indirect/src/std_model.rs#L49)、[:94](../../scene/rendering/gpu-indirect/src/std_model.rs#L94)）让「逐个尝试、第一个 Some 生效」的链式分发成为惯例——正是 picker 链的同款模式。viewer 里四个渲染器被装进一个 vec（[frame_all.rs:339](../../application/viewer-content/src/rendering/frame_all.rs#L339)），`IndirectPreferredComOrderRenderer`（[scene_model.rs:57](../../scene/rendering/gpu-indirect/src/scene_model.rs#L57)）把它们与节点渲染包成 `IndirectBatchSceneModelRenderer`，`render_indirect_batch_models`（[scene_model.rs:95](../../scene/rendering/gpu-indirect/src/scene_model.rs#L95)）按绑定索引组装 RenderArray：

```text
draw_source(provider) + tex + pass + midc 降级 + model_info(空) + shape + node + camera + material(空)
```

扩展的 shape 组件（宽线/宽点/文字的 `shape_renderable_indirect` 返回体）就在这里与相机、pass 合成，`TraditionalDraw(provider.draw_command())` 提交。

### mid：命令生成

四个扩展的命令生成全部收敛到 `use_and_create_default_indirect_draw_provider`（[gpu-base/src/mid/mod.rs:84](../../scene/rendering/gpu-base/src/mid/mod.rs#L84)）：compute pass 逐实体调用 builder 的 `generate_draw_command` 写 `INDIRECT` 缓冲，按子列表切片成 provider（原生 `MultiIndirectDrawBatch` 或降级 `MIDCDowngradeBatch`，见 [indirect-draw-command-guide.md](indirect-draw-command-guide.md)）。扩展侧需要保证的只有「`generate_draw_command` 与 `draw_command_host_access` 的 vertex_count 公式一致」与「分配失败 count 为 0」。

### geometry-query：拾取与包围盒

拾取：局部 picker 链（viewer pick.rs）+ 各扩展 `LocalModelPicker`，世界层复用；屏幕空间容差让宽线/宽点拾取手感恒定。包围盒：`SceneModelLocalBounding` 合并四个扩展的局部包围盒（bounding.rs），`SceneModelWorldBounding` 乘世界矩阵后供 `SceneModelPickerBaseImpl` 预筛与 `SceneDynamicBvhIterProvider`（[dynamic-bvh-scene](../../extension/dynamic-bvh-scene/src/iter.rs)）加速——宽线/宽点的 BVH margin 额外叠加容差换算（[pick.rs:69](../../application/viewer-content/src/pick.rs#L69)）。注意拾取与渲染完全解耦：拾取读数据库，渲染读 GPU 池，二者只通过「相同的数据语义」保持一致。

### GLES（host）路径

GLES 后端没有间接命令生成能力，走 `GLESModelRenderImpl`（[gpu-gles/src/std_model.rs:3](../../scene/rendering/gpu-gles/src/std_model.rs#L3)）：`shape_renderable` 返回「组件 + DrawCommand」对，每实体逐个提交。wide-line（[gles_draw.rs:77](../../extension/wide-line/src/gles_draw.rs#L77)）、wide-points（[gles_draw.rs](../../extension/wide-styled-points/src/gles_draw.rs)）、text-3d（[gles_draw.rs:44](../../extension/text-3d/src/gles_draw.rs#L44)）各自实现了宿主侧数据展开（宽线展开段实例、文字打包字形图集），viewer 在 GLES 分支装配（[frame_all.rs:166](../../application/viewer-content/src/rendering/frame_all.rs#L166)）。cell-mesh 无 GLES 实现。host-driven indirect 模式（GL 后端用间接 API）则复用同一套 builder 的 `draw_command_host_access`（见 builder 一节）。

## 使用模板

### 模板一：创建宽线模型

[application/viewer/src/viewer/test_content/widen_line.rs:5](../../application/viewer/src/viewer/test_content/widen_line.rs#L5) 是完整范例：

```rust
let mut writer = global_entity_of::<WideLineModelEntity>().entity_writer();
let mesh_buffer = build_wide_line_mesh(|builder| { /* 用 AttributesLineMeshBuilder 生成线段 */ });
let wide_line_model = writer.new_entity(|w| {
  w.write::<WideLineWidth>(&5.)
    .write::<WideLineStylePattern>(&0xffc0)
    .write::<WideLineStyleFactor>(&6.0)
    .write::<WideLineMeshBuffer>(&mesh_buffer)
});
let child = s_writer.create_root_child();
s_writer.set_local_matrix(child, Mat4::translate((5., 0., 0.)));
s_writer.model_writer.new_entity(|w| {
  w.write::<SceneModelWideLineRenderPayload>(&wide_line_model.some_handle())
    .write::<SceneModelBelongsToScene>(&scene.some_handle())
    .write::<SceneModelRefNode>(&child.some_handle())
});
```

与标准模型的差别只有 payload 组件换成了 `SceneModelWideLineRenderPayload`；挂载后 key 分桶、拾取、包围盒全部自动生效。宽点/文字同理（`SceneModelWideStyledPointsRenderPayload` / `SceneModelText3dPayload`）。cell-mesh 则挂在 StandardModelEntity 上（`StandardModelCellMeshPayload`），外观与标准模型一致。

### 模板二：接入一个新的扩展几何类型

把四件套补齐即可，其余链路自动适配：

| 件 | 实现 | 接入点 |
| --- | --- | --- |
| 数据模型 | `register_*_data_model` + payload 外键 + 组件 | viewer `lib.rs` 注册 |
| key | `use_*_group_key` 返回 `ForeignHash{TypeId, 状态}` | `GroupKeyForeignImpl` 对应槽位 |
| 渲染器 | `IndirectModelRenderImpl`（独立类型）或 `IndirectModelShapeRenderImpl`（std model 形状） | frame_all.rs 的渲染器 vec / shape vec |
| picker | `LocalModelPicker` + `AbstractMesh` 视图 | viewer pick.rs 的局部链 |
| 包围盒 | `SharedResultProvider` 或等价函数 | bounding.rs 的 select 链 |

关键约束：key、PSO 哈希、实现分类 key 三者哈希内容必须互洽；几何池失败时 count 必须为 0；`require_alpha_blend` 决定透明过滤归属。

## 常见疑问

- **为什么顶点数用 stride 整数倍、而不是把计数逻辑写进 shader 条件分支？** 命令生成发生在独立 compute pass，每实体一行命令；stride 是编译期常量（`val(18)` / `val(6)` / `val(2)`），顶点着色器用 `vertex_index / stride`、`vertex_index % stride` 分解单元与单元内顶点，零分支。若 stride 需要变化（如宽线的 native 开关），必须把它升级为「实现分类 key」的一部分（`get_impl_distinguish_key_by_impl_select_id`），让不同 stride 的实体分到不同子列表、不同命令、不同 PSO——宽线正是这么做的。
- **为什么 key 里不哈希颜色/样式/文字内容？** PSO 只关心「管线长什么样」：深度、混合、拓扑、shader 代码。颜色等是数据，变化时走参数行/几何池的增量写入，换管线毫无必要。硬塞进 key 会把性能从「稀疏写」拖成「换子列表」。
- **为什么 cell-mesh 不做一个独立的 `IndirectModelRenderImpl`？** 单元网格需要材质（双面颜色是顶点数据，但材质类型、状态覆盖、蒙皮都要标准路径），挂在 shape 槽上直接获得 `SceneStdModelIndirectRenderer` 的全部能力；代价是它不能像宽线那样自带「非标准渲染管线」（如无材质自定义 shader 或自定义混合）。需要独立渲染管线的扩展走 model 槽，需要标准材质的几何走 mesh 槽。
- **cell-mesh 在 GLES 后端不可见怎么办？** 这是当前的设计空缺：cell-mesh 没有 `gles_draw.rs`，GLES 后端没有 `GLESModelRenderImpl` 能处理它——`GLESSceneRenderer`（[gpu-gles/src/scene.rs:27](../../scene/rendering/gpu-gles/src/scene.rs#L27)）逐实体渲染时 cell-mesh 模型的哈希失败，被 `SceneModelErrorRecorder` 记录并跳过。key 组装只发生在 [frame_all.rs:387](../../application/viewer-content/src/rendering/frame_all.rs#L387) 的 indirect 分支（GLES 分支不建增量 device 提取器）；但**拾取不受后端影响**：`use_cell_mesh_picker` 总在 CPU 拾取链里注册，GLES 下 cell-mesh 模型仍可被射线/视锥拾取到。host-driven indirect 模式可以渲染 cell-mesh（indirect 分支里渲染器照常注册，builder 走 host 表达）。
- **`base_instance = draw_id` 在顶点着色器里为什么读不到？** `DrawIndirectArgsStorage.base_instance` 会作为实例号注入顶点阶段，但扩展的顶点展开只使用 `VertexIndex`（`instance_index = vertex_index / stride` 是单元下标，不是实体下标）；实体的全局身份始终来自 `LogicalRenderEntityId`（provider 注册）→ `sm_to_xxx_device`。`base_instance` 的值因此对渲染无影响，保留实体下标只是方便调试。
- **init-only 参数（如 `use_native_line_for_one_width_line`）为什么不能运行期改？** 它同时进了增量 key（host 侧哈希）、实现分类 key 与 PSO 哈希，而这三处都不是组件增量查询驱动的——运行期翻转它不会触发任何重新分桶/换管线，会直接破坏「子列表共享 PSO」的不变量。它是「进程生命周期内固定」的配置，代码注释里反复强调。
- **为什么宽点/文字的拾取要造三角形而不是直接按点/包围盒判距离？** 统一走 `AbstractMesh` + `ray_intersect_*` 通道可以免费获得「命中点、primitive 索引、最近命中」全套语义（`frustum_query_sub_primitives` 也复用同一视图）；宽点需要在屏幕空间膨胀（NDC 构造），文字需要局部矩阵，三角形化是两者共同的表达方式。代价是每命中一次生成临时三角形（`primitive_at` 现算），对拾取规模（每次点击一次）可忽略。

## 延伸阅读

- 批提取、`GroupKeyForeignImpl` 与 id 池：[batch-extractor-guide.md](batch-extractor-guide.md)
- 批 → 实现分类 → provider → 渲染的帧内链路：[gpu-indirect-batch-collector-guide.md](gpu-indirect-batch-collector-guide.md)
- mid 层命令生成与 MIDC 降级：[indirect-draw-command-guide.md](indirect-draw-command-guide.md)
- 属性网格作为「标准形状」的对照实现：[attribute-mesh-indirect-render-guide.md](attribute-mesh-indirect-render-guide.md)
- 拾取分层与容差换算：[geometry-query-guide.md](geometry-query-guide.md)
- 标准模型渲染实现与 id 注入：[material-indirect-render-guide.md](material-indirect-render-guide.md)
- 两阶段 hook 与共享计算：[query-hook-guide.md](query-hook-guide.md)、[hooks-guide.md](hooks-guide.md)
- GPU 池化分配工具（`use_range_allocated_device_buffers` / `use_db_device_foreign_key`）：[platform/graphics/webgpu-hook-utils/src/lib.rs](../../platform/graphics/webgpu-hook-utils/src/lib.rs)
- 范围分配器：[utility/growable-range-allocator/src/lib.rs](../../utility/growable-range-allocator/src/lib.rs)
