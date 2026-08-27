# Rendiation 场景拾取（pick）与几何查询指南（scene/geometry-query）

本文梳理 [scene/geometry-query](../../scene/geometry-query) 的 3D 场景拾取抽象体系：射线拾取（ray cast）、视锥查询（frustum query）、两者共享的容差与预筛机制，以及从数据库几何数据到命中点的完整流水线。

## 前置阅读

拾取查询直接建立在场景数据模型、增量查询和关系数据库之上，建议先了解：

| 文档 | 内容 |
| --- | --- |
| [skill-translation/scene-core-structure-zh.md](skill-translation/scene-core-structure-zh.md) | 场景实体类型（SceneModelEntity、StandardModelEntity、节点与变换） |
| [skill-translation/database-schema-zh.md](skill-translation/database-schema-zh.md) | 类型安全关系数据库：外键、组件读视图 |
| [skill-translation/query-system-zh.md](skill-translation/query-system-zh.md) | 增量查询（DualQuery）与扇出 |
| [query-hook-guide.md](query-hook-guide.md) | 两阶段执行模型（spawn/resolve）、UseResult、共享计算 |
| [hooks-guide.md](hooks-guide.md) | hook 运行时与状态管理 |
| [skill-translation/rendiation-algebra-zh.md](skill-translation/rendiation-algebra-zh.md) | Scalar、向量/矩阵、SpaceEntity（变换） |

## 模式概览

交互式 3D 应用的核心问题之一：鼠标点在哪里、框选了哪些物体。rendiation 用纯 CPU 几何查询回答这个问题——拾取系统与 GPU 渲染管线完全解耦，直接从数据库里读几何数据做求交：

- **射线拾取**：从相机发出的世界空间射线（由鼠标位置反投影得到），找出沿途命中的模型（最近命中或全部命中）。
- **视锥查询**：屏幕上一个矩形框反投影成世界空间视锥，找出与视锥相交或包含于视锥的模型；还能细到 primitive 级（命中哪些三角形/线段/点）。
- 两者共用一个三层 trait 抽象：顶层候选集迭代（TLAS）、世界空间模型级查询（SceneModelPicker）、局部空间 primitive 级查询（LocalModelPicker）。
- 查询结果与"可选、可见"预筛、屏幕空间容差（宽线/宽点按像素拾取）深度耦合，这套机制也是框架的核心价值。

## 核心概念

| 概念 | 定义 | 说明 |
| --- | --- | --- |
| `SceneRayQuery` | [scene/geometry-query/src/lib.rs](../../scene/geometry-query/src/lib.rs) | 一次射线查询的全部输入：世界射线、相机上下文、额外屏幕容差、可选的命中过滤器 |
| `SceneFrustumQuery` | 同上 | 一次视锥查询的全部输入：世界视锥、精确测试缓存、相机上下文、额外容差 |
| `CameraQueryCtx` | 同上 | 相机换算上下文：逻辑像素视口尺寸、pixels_per_unit 计算、世界矩阵与 VP 矩阵、最大缩放 |
| `SceneModelPicker` | [scene/geometry-query/src/scene_model.rs](../../scene/geometry-query/src/scene_model.rs) | 世界空间模型级查询 trait，四个方法：ray_query_nearest / ray_query_all / frustum_query / frustum_query_sub_primitives |
| `LocalModelPicker` | [scene/geometry-query/src/model.rs](../../scene/geometry-query/src/model.rs) | 局部空间 primitive 级查询 trait，内部还负责报告"点/线"所需的拾取容差 |
| `SceneModelIterProvider` | [scene/geometry-query/src/iter.rs](../../scene/geometry-query/src/iter.rs) | 候选模型集迭代器（TLAS 抽象），ray 与 frustum 各一个工厂方法 |
| `SceneModelPickerBaseImpl<T>` | [scene/geometry-query/src/scene_model.rs](../../scene/geometry-query/src/scene_model.rs) | 把 `T: LocalModelPicker` 提升为世界空间 SceneModelPicker 的样板实现 |
| `SceneModelSelectable` | [scene/geometry-query/src/lib.rs](../../scene/geometry-query/src/lib.rs) | SceneModelEntity 上的 bool 组件，默认 true，决定是否参与拾取 |
| `ObjectTestPolicy` | [scene/geometry-query/src/scene_model.rs](../../scene/geometry-query/src/scene_model.rs) | 视锥测试语义：`Intersect`（相交即可）或 `Contains`（完全包含） |
| `IntersectTolerance` / `ToleranceType` | [content/mesh/core/src/feature/intersection.rs](../../content/mesh/core/src/feature/intersection.rs) | 拾取容差（局部空间或屏幕空间），点/线求交的距离阈值 |
| `MeshBufferHitPoint` | 同上 | 命中结果：HitPoint3D + primitive 索引 |
| `FrustumIntersectionTestHelper` | [scene/geometry-query/src/frustum.rs](../../scene/geometry-query/src/frustum.rs) | 视锥角点与去重边方向的 SAT（分离轴）精确测试缓存 |

## 为什么需要这套分层

先看数据流才能理解分层动机。一个 SceneModelEntity 只是场景图中的一个占位（[scene/core/src/model.rs](../../scene/core/src/model.rs)）：它通过 `SceneModelRefNode` 挂在节点上获得世界矩阵，通过 `SceneModelBelongsToScene` 归属场景，再通过各类 payload 外键指向真正的几何数据——`SceneModelStdModelRenderPayload` 指向 StandardModelEntity（含 AttributesMeshEntity 网格），宽线/宽点/文字/单元网格扩展则各有自己的 payload 外键。

拾取要回答的层级因此自然分层：

```text
场景（SceneEntity）
  └─ 候选模型集（SceneModelIterProvider，TLAS）
       └─ SceneModelEntity（世界空间查询，SceneModelPicker）
            ├─ 世界矩阵 / 可选可见性 / 世界包围盒（SceneModelPickerBaseImpl）
            └─ 几何数据（LocalModelPicker，局部空间）
                 └─ primitive 求交（mesh_core AbstractMeshIntersectionExt）
                      └─ 数学求交（rendiation_geometry IntersectAble）
```

- 局部空间的求交在 f32 下进行（GPU 数据本来就在 f32），世界空间的矩阵与射线在 f64 下进行（避免大尺度场景下的精度损失）。
- 一个 SceneModelEntity 可能有多种 payload，也可能一个都查不到（比如只挂了没有拾取实现的扩展）——所以各层都用 `Option` 表达"我不处理这个模型"，调用链逐层 fall through。
- 每层职责单一，新几何类型（新的扩展模型）只需要实现局部层，世界层与候选集层完全复用。

## 查询输入：SceneRayQuery / SceneFrustumQuery / CameraQueryCtx

```rust
pub struct SceneRayQuery<'a> {
  pub world_ray: Ray3<f64>,
  pub camera_ctx: CameraQueryCtx,
  pub extra_screen_space_tolerance: f32,
  pub filter: Option<&'a SceneModelPickFilter<'a>>,
}

pub struct SceneFrustumQuery {
  pub world_frustum: Frustum<f64>,
  pub world_helper: Option<FrustumIntersectionTestHelper<f64>>,
  pub camera_ctx: CameraQueryCtx,
  pub extra_screen_space_tolerance: f32,
}
```

`filter` 是 `dyn Fn(&MeshBufferHitPoint<f64>, EntityHandle<SceneModelEntity>) -> bool`——命中后保留与否的最终裁决。典型用途见 [effect/plane_array_clip/src/lib.rs](../../effect/plane_array_clip/src/lib.rs) 的 `ArrayClipPickFilter`：把被裁剪平面切掉的命中点过滤掉（返回 true 保留）。

`CameraQueryCtx` 承载"屏幕空间容差换算成局部空间容差"所需的全部相机信息：

- `camera_view_size_in_logic_pixel`：视口逻辑像素尺寸（除以 device pixel ratio 后的值）。
- `pixels_per_unit_calc`：`Box<dyn Fn(f32, f32) -> f32>`，输入"相机到目标点的投影距离"与"视口逻辑像素高"，输出"该距离处一个世界单位占多少像素"。来自 [extension/gui-3d/src/lib.rs](../../extension/gui-3d/src/lib.rs) 的 `ViewportPointerCtx::create_ratio_cal`，透视投影与正交投影各自实现。
- `camera_world` / `camera_vp` / `camera_max_scale`：世界矩阵、VP 矩阵、世界矩阵最大缩放。`camera_vp` 用于宽点拾取把局部坐标投影到 NDC 做屏幕空间膨胀；`camera_max_scale` 用于正交相机（注释特别指出：用户常用 scale 控制正交相机视野范围，容差换算必须考虑它）。

换算函数 `compute_local_tolerance`（[lib.rs](../../scene/geometry-query/src/lib.rs)）分两步：

```text
local_tolerance = tolerance.value * camera_max_scale / target_world_mat_max_scale
若 ToleranceType::ScreenSpace：
  local_tolerance /= pixel_per_unit（投影距离处每单位像素数）
```

- `LocalSpace` 型容差只按物体的世界矩阵最大缩放缩放（物体被放大，局部容差相应缩小才等价）。
- `ScreenSpace` 型容差先换算到世界尺度，再除以"该距离处每单位多少像素"，得到物体局部空间的像素等价长度——这样宽线/宽点无论相机远近、物体缩放多大，拾取手感都恒定在屏幕像素上。

## SceneModelPicker 层：世界空间模型查询

### trait 与 Option 语义

[scene_model.rs](../../scene/geometry-query/src/scene_model.rs) 定义四个方法，请求对象都携带 `idx`（SceneModelEntity 句柄）、`override_world_mat`（可替换节点矩阵，装饰器用）、`ignore_pre_check`（跳过可选/可见预筛）与各自的查询上下文：

- `ray_query_nearest` → `Option<MeshBufferHitPoint<f64>>`：世界空间最近命中（distance 已用世界坐标重算）。
- `ray_query_all` → `Option<()>`：把全部命中 push 进 `results`（f64 世界空间），`local_result_scratch` 是 f32 局部空间的复用缓冲。
- `frustum_query` → `Option<bool>`：是否满足 policy。
- `frustum_query_sub_primitives` → `Option<()>`：把命中的 primitive 索引 push 进 `results`。

**Option 语义贯穿全框架：`None` 表示"此实现不处理该模型"（数据不存在或错误），`Some` 表示"处理完成，结果在返回值/缓冲里"。** 由此 `Vec<Box<dyn SceneModelPicker>>` 与 `Vec<Box<dyn LocalModelPicker>>` 的链式实现（各文件中的 for 循环逐个尝试、第一个返回 Some 即终止）成为组合多种几何类型的标准手段——每个局部 picker 只认自己的 payload 外键，查不到就返回 None 让下一个试。

### SceneModelPickerBaseImpl：世界层样板

`SceneModelPickerBaseImpl<T: LocalModelPicker>` 是核心复用件。它持有 `util: SceneModelPickerBaseImplUtil`（一组数据库查询视图）与 `internal: T`：

```rust
pub struct SceneModelPickerBaseImplUtil {
  pub node_world: BoxedDynQuery<EntityHandle<SceneNodeEntity>, Mat4<f64>>,
  pub node_net_visible: BoxedDynQuery<EntityHandle<SceneNodeEntity>, bool>,
  pub scene_model_node: ForeignKeyReadView<SceneModelRefNode>,
  pub sm_world_bounding: BoxedDynQuery<EntityHandle<SceneModelEntity>, Option<Box3<f64>>>,
  pub sm_local_bounding: BoxedDynQuery<EntityHandle<SceneModelEntity>, Box3<f32>>,
  pub selectable: ComponentReadView<SceneModelSelectable>,
}
```

`pre_check(idx, ignore_pre_check)` 做两层预筛后返回节点句柄：

- `SceneModelSelectable` 为 false 的模型直接跳过（注意 `SceneModelSelectable` 默认 true，见 [lib.rs](../../scene/geometry-query/src/lib.rs) 的 `declare_component!` 与 `register_selectable_data_model()`；C API 侧通过 `scene_model_set_selectable` 写入，见 [c_api/model.rs](../../application/viewer-content-api/src/c_api/model.rs)）。
- 节点 `node_net_visible` 为 false（自身或祖先不可见）跳过。

`get_mat_and_world_aabb` 取世界矩阵与包围盒：世界包围盒优先用 `sm_world_bounding`（共享计算给出的缓存，动态模型为 None），否则用 `sm_local_bounding` 乘世界矩阵现算。

`ray_query_nearest` 的完整流水线：

- `pre_check` 过滤可选/可见。
- 取世界矩阵与包围盒（有 `override_world_mat` 时用它替换节点矩阵，包围盒用局部包围盒乘该矩阵现算）。
- `pre_check_bounding_early_return_and_compute_local_tolerance`（[scene_model.rs](../../scene/geometry-query/src/scene_model.rs)）：
  - 先问 `internal.bounding_enlarge_tolerance(idx)`——点/线类几何需要容差才能被拾取到（返回 `Some(None)` 表示"我处理但不需要容差"，`None` 表示"不处理"）。
  - 算局部容差：自身容差（若为 ScreenSpace 型经 `compute_local_tolerance` 换算）加上 `ctx.extra_screen_space_tolerance` 的换算值。
  - 容差大于 0 时把世界包围盒放大 `local_tolerance * max_scale`。
  - 世界射线与世界包围盒求交（[math/geometry/src/lib.rs](../../math/geometry/src/lib.rs) 的 `IntersectAble`，`Ray3 vs Box3`），未命中直接返回 None——这是最重要的性能关卡。
- 世界射线经 `mat.inverse_or_identity()` 变换到局部空间、降为 f32，连同局部容差交给 `internal.ray_query_local_nearest`。
- 命中点（局部 f32）经 `transform_hit_point_to_world` 变换回世界 f64，distance 按"射线原点到世界命中点"重算。
- 应用 `ctx.filter`，被拒绝则返回 None。

`ray_query_all` 同理，局部结果先全部进 f32 scratch，统一变换后逐个过 filter 再 push 到 f64 results。

### frustum 查询：复用同一套准备逻辑

`prepare_frustum_test` 完成与射线路径几乎相同的准备：pre_check、取矩阵与包围盒、然后：

```rust
let frustum = request.frustum.world_frustum
  .apply_matrix_into(mat.inverse_or_identity());   // 世界视锥 → 局部空间
let frustum = frustum.into_f32();                  // f64 → f32
let helper = FrustumIntersectionTestHelper::new(&frustum); // 精确测试缓存
```

局部空间视锥随后交给 `internal.frustum_query_local`。**frustum 与 ray 共用世界层（矩阵、预筛、包围盒），差异只在局部测试本身**——这就是任务要突出的"frustum 复用 ray pick 框架"：差别仅在 `LocalModelPicker` 的局部测试方法。

### FrustumIntersectionTestHelper：SAT 精确测试

[frustum.rs](../../scene/geometry-query/src/frustum.rs) 提供三件工具：

- `FrustumIntersectionTestHelper::new`：从 6 个平面相交出 8 个角点，提取 12 条边的方向并去重（平行边只留第一条）。退化视锥（某平面组合无交点）返回 None。
- `frustum_intersect_aabb` / `frustum_intersect_line_segment` / `frustum_intersect_triangle`：分离轴测试（SAT），依次检查 AABB 三个轴、视锥 6 个平面法线（p-vertex 测试）、视锥边 × 被测体边的叉积轴。`helper` 为 None 时回退到保守 p-vertex 测试（无假阴性、可能有假阳性），所以"精确测试"是可选开关。
- `frustum_intersect_aabb` 是 `SceneFrustumQuery.world_helper`（f64 世界空间）与动态 BVH 遍历的共用件。

`frustum_test_abstract_mesh` 把 policy 展开为 primitive 级聚合：`Intersect` 是 `any(tester)`，`Contains` 是 `all(tester)`。primitive 级测试语义（`frustum_test_primitive`）：点要求被视锥包含；线段 `Intersect` 用 SAT 线段测试、`Contains` 要求两端都在内；三角形 `Intersect` 用 `frustum_intersect_triangle`、`Contains` 要求三顶点都在内。

## LocalModelPicker 层：局部空间 primitive 查询

trait 五个方法（[model.rs](../../scene/geometry-query/src/model.rs)）：

| 方法 | 职责 |
| --- | --- |
| `bounding_enlarge_tolerance` | 报告该几何是否需要拾取容差：`Some(Some(tol))` 需要、`Some(None)` 不需要、`None` 不处理 |
| `ray_query_local_nearest` | 局部射线最近命中，返回局部空间结果 |
| `ray_query_local_all` | 全部命中 push 进请求的 `results`（f32 局部空间） |
| `frustum_query_local` | 局部视锥测试（含 `helper` 与 `policy`） |
| `frustum_query_local_sub_primitives` | 命中的 primitive 索引 push 进 `results` |

局部求交的底层是 mesh_core 的 `AbstractMeshIntersectionExt`（[content/mesh/core/src/feature/intersection.rs](../../content/mesh/core/src/feature/intersection.rs)）：任何实现 `AbstractMesh`（[container/mod.rs](../../content/mesh/core/src/container/mod.rs)，随机访问的 primitive 迭代器，只需 `primitive_count` + `primitive_at`）的类型，只要 `Ray3: IntersectAble<Primitive, OptionalNearest<HitPoint3D>, C>`，就能免费得到 `ray_intersect_iter` / `ray_intersect_all` / `ray_intersect_nearest`。

数学求交在 rendiation_geometry（[dimension3/intersection.rs](../../math/geometry/src/dimension3/intersection.rs)）：

- 三角形：Möller 风格射线-三角形测试，参数 `FaceSide`（Double 不剔除背面；拾取统一用 `FaceSide::Double`）。
- 线段/点：参数是**容差** `t`——求射线到线段/点的最近距离，距离平方小于 `t²` 即命中。这就是 `IntersectTolerance` 的消费端。
- 包围盒：求最近命中点（`OptionalNearest<HitPoint3D>`）或仅判断是否命中（`bool`）。

### AttributeMeshPicker：标准网格

[model.rs](../../scene/geometry-query/src/model.rs) 的 `AttributeMeshPicker` 是网格模型的局部实现，由 `use_attribute_mesh_picker`（hook，见 [query-hook-guide.md](query-hook-guide.md) 的两阶段模式）组装：读 `SceneModelStdModelRenderPayload` → `StandardModelRefAttributesMeshEntity` 找到网格实体，再从网格实体的顶点缓冲区关系里挑出 `AttributeSemantic::Positions` 语义的缓冲区、按索引格式（u16/u32）构建 `AttributesMeshEntityAbstractMeshReadView`（[container/attributes/access.rs](../../content/mesh/core/src/container/attributes/access.rs)，把 `MeshPrimitiveTopology` 展开为 `AttributeDynPrimitive`：点/线段/三角形）。

要点：

- 拾取配置是 `MeshBufferIntersectConfig { tolerance_local, triangle_face: FaceSide::Double }`（[container/attributes/picking.rs](../../content/mesh/core/src/container/attributes/picking.rs)）。
- `bounding_enlarge_tolerance` 按拓扑返回容差：`PointList` 用 `pick_point_tolerance`、`LineList`/`LineStrip` 用 `pick_line_tolerance`，两者默认都是 `IntersectTolerance::new(1.0, ToleranceType::ScreenSpace)`（屏幕空间 1 像素）；三角形返回 `Some(None)`（不需要容差）。
- 顶点数据经 `SceneBufferViewReadView`（[scene/core/src/reader.rs](../../scene/core/src/reader.rs)）从 `BufferEntityData` 里按字节切片读出。

### 无网格模型：extension 中的局部实现

每个扩展几何类型都是"实现 `AbstractMesh` 视图 + 实现 `LocalModelPicker`"的相同套路：

| 实现 | 位置 | 几何来源 | 拾取几何 |
| --- | --- | --- | --- |
| `WideLinePicker` | [extension/wide-line/src/pick.rs](../../extension/wide-line/src/pick.rs) | `WideLineMeshBuffer` 顶点对 | `LineSegment`（line strip 按相邻顶点对）；容差 = 线宽一半（ScreenSpace） |
| `WidePointsPicker` | [extension/wide-styled-points/src/pick.rs](../../extension/wide-styled-points/src/pick.rs) | `WideStyledPointsMeshBuffer` 点 + 宽度 | 每个点经 `camera_vp` 投影到 NDC、按像素宽膨胀成两个三角形再投影回局部空间；容差 = 最大点宽（ScreenSpace） |
| `TextPicker` | [extension/text-3d/src/pick.rs](../../extension/text-3d/src/pick.rs) | 文字 `SlugBuffer` 的逐字 hit_boxes | 每个 bbox 两个三角形，乘 `Text3dLocalTransform` |
| `CellMeshPicker` | [extension/cell-mesh/src/pick.rs](../../extension/cell-mesh/src/pick.rs) | 单元网格 `CellMeshUnitsBuffer`（p1..p4 + center） | 每单元两个三角形（按 shrink_ratio 收缩） |

注意 `WidePointsPicker` 是唯一"需要世界矩阵做屏幕空间膨胀"的局部实现——它利用 `LocalRayQueryRequest.world_mat` 与 `camera_ctx.camera_vp` 构造 local↔NDC 双向变换，演示了 `LocalModelPicker` 请求里为什么带 `world_mat` 与 `camera_ctx`（广角下四边形在局部空间并不真实，但投影回屏幕恰好是正方形）。

## 装饰器 picker：实例化与视依赖变换

### TransformInstancedMeshPicker：实例化模型

[extension/transform-instanced-model/src/pick.rs](../../extension/transform-instanced-model/src/pick.rs) 是 `SceneModelPicker` 上的装饰器：

- 先问 `internal`（非实例化路径），返回 Some 即终止；None 再走实例化路径——实例化模型可以"既有点击源模型几何的路径、又有实例路径"。
- 数据链：`SceneModelTransformInstancedModelPayload` → 实例化模型实体 → `TransformInstancedModelRefSceneModel` 指向**源模型**（一个普通 SceneModelEntity）+ `TransformInstancedModelPerUnitTransform` + `TransformInstancedModelInstanceBuffer`（`&[Mat4<f32>]` 实例矩阵列表）。
- `iter_mats` 合成每个实例的世界矩阵：`instance_own_transform * (instance_mat * per_unit_transform)`。
- 对每个实例，用 `override_world_mat: Some(&m)` 重新查询源模型（`ignore_pre_check: true` 跳过已做过的预筛），**命中结果的 `primitive_index` 被覆写为实例索引**——上层据此知道点中了哪个实例。
- frustum 语义：`Intersect` 是"任一实例相交"，`Contains` 是"全部实例被包含"；`frustum_query_sub_primitives` 返回命中实例的索引列表。

注释还诚实说明局限：不做"实例缓冲整体包围盒"预筛（每个实例逐次走源模型的包围盒 early-out），未来可优化。

### SceneModelPickerWithViewDep：视依赖变换

[extension/view-dependent-transform/src/picking.rs](../../extension/view-dependent-transform/src/picking.rs) 解决一类特殊模型：它的世界矩阵不来自节点，而是随视角变化（如 occlusion 风格化变换、视点相关形变）。装饰器按 `active_view`（viewport id）查 `view_mats: BoxedDynQuery<ViewSceneModelKey, Mat4<f64>>`，把查到的矩阵作为 `override_world_mat` 注入 internal——于是世界层完全不需要知道矩阵来自节点还是视依赖计算。

## SceneModelIterProvider：候选模型集的 TLAS

ray pick 与 frustum pick 的顶层驱动（[iter.rs](../../scene/geometry-query/src/iter.rs)）：

| 函数 | 语义 |
| --- | --- |
| `pick_models_nearest` | 遍历候选集，保留距离最近的命中，返回 `(HitPoint3D<f64>, EntityHandle<SceneModelEntity>)` |
| `pick_models_all` | 全部命中与对应的模型句柄（按 hit 数对齐 push） |
| `range_pick_models` | 视锥查询，命中的模型经回调收集 |

候选集来自 `SceneModelIterProvider`（"TLAS 抽象"），两种实现：

- `NaiveSceneModelIterProvider`（[application/viewer-content/src/pick.rs](../../application/viewer-content/src/pick.rs)）：`SceneModelBelongsToScene` 反向外键，全量遍历——正确性基准。
- `SceneDynamicBvhIterProvider`（[extension/dynamic-bvh-scene/src/iter.rs](../../extension/dynamic-bvh-scene/src/iter.rs)）：动态 BVH 加速。ray 遍历对每个节点做"膨胀后的 AABB 与射线相交"测试（膨胀量 = 节点 margin + `extra_screen_space_tolerance` 经 `compute_local_tolerance` 换算）；frustum 遍历用 `f_intersect_exact` 三分（Inside 子树整体收集 / Intersect 继续下钻 / Outside 剪枝），Inside 通过 n-vertex 测试快速判定。

## 下游组装：ViewerPicker

[application/viewer-content/src/pick.rs](../../application/viewer-content/src/pick.rs) 的 `use_viewer_scene_model_picker_impl` 把所有片段组合成 `ViewerPicker`：

```text
Vec<Box<dyn LocalModelPicker>>（attribute / wide_line / wide_point / text / cell_mesh）
  └─ SceneModelPickerBaseImpl（+ SceneModelPickerBaseImplUtil，读全局查询视图）
       └─ TransformInstancedMeshPicker
            └─ SceneModelPickerWithViewDep（view-dependent 矩阵覆盖）
                 └─ Box<dyn SceneModelPicker>
+ SceneModelIterProvider（Naive 或 SceneDynamicBvhIterProvider，按 use_scene_bvh 开关）
+ camera_transforms / ndc / clip_filter（ArrayClipPickFilter）
```

同一文件还提供两个输入转换函数：

- `create_viewport_pointer_ctx`：鼠标逻辑像素坐标 → 找最上层 viewport → 用 `CameraTransform.view_projection_inv` 反投影出世界射线（`cast_world_ray`，见 [scene/core/src/camera.rs](../../scene/core/src/camera.rs)），并组装 `ViewportPointerCtx`（射线、视口尺寸、投影矩阵、`create_ratio_cal` 闭包）。
- `create_range_pick_frustum`：屏幕两个角点 → `Frustum::new_from_matrix_ndc`（[dimension3/frustum.rs](../../math/geometry/src/dimension3/frustum.rs)，按给定 NDC 矩形裁剪 6 个平面）→ `SceneFrustumQuery`；`precise_intersection_test` 开关决定是否构造 `world_helper`。

viewer 侧（[application/viewer/src/viewer/pick.rs](../../application/viewer/src/viewer/pick.rs)）的 `ViewerPickerWithCtx` 把它与当前指针上下文绑在一起，实现 `Picker3d` trait，并暴露便捷方法：`pick_range`、`pick_model_nearest_all`、`pick_models_list_all`、`pick_model_nearest`（单模型）、`pick_models_nearest` 等。`use_viewer_scene_model_picker` 只在 `EventHandling` 阶段构造 picker（其余阶段返回 None），并按当前 viewport 调用 `set_active_view`。

## 下游消费场景

| 消费点 | 位置 | 用途 |
| --- | --- | --- |
| 场景拾取调试面板 | [application/viewer/src/viewer/feature/pick_scene.rs](../../application/viewer/src/viewer/feature/pick_scene.rs) | 左键点击最近拾取（写入选 `viewer.selection.selected_model`）、按住 A 键列出全部命中、按 Q 键拖框做范围拾取（`Contains`/`Intersect` 可切换）、GPU id buffer 拾取可选路径、BVH 线框调试 |
| 3D widget 交互 | [application/viewer/src/viewer/widget_bridge.rs](../../application/viewer/src/viewer/widget_bridge.rs) | `prepare_picking_state` 对 widget 模型组做最近拾取，产出 `Interaction3dCtx` 驱动 gizmo 拖拽 |
| 相机辅助线 | [application/viewer/src/viewer/feature/camera_helper.rs](../../application/viewer/src/viewer/feature/camera_helper.rs) | 拾取相机模型上被点中的 primitive（辅助线索引），按 `primitive_index` 做偏移 |
| C API 查询 | [application/viewer-content-api/src/viewer_api.rs](../../application/viewer-content-api/src/viewer_api.rs) | `ViewerQueryAPI::pick_list`（射线全部命中 + 可选 clip 过滤）、`pick_range`（视锥范围）、`pick_range_sub_primitive`（指定模型的 primitive 级范围命中）；C 边界在 [c_api/viewer.rs](../../application/viewer-content-api/src/c_api/viewer.rs) |

值得注意：scene/rendering 下**不消费** geometry-query——渲染侧有自己的 GPU 剔除（如 frustum-culling）走独立路径，拾取是完全独立的 CPU 几何查询体系，二者不共享代码。

## 使用模板

### 模板一：为新的扩展几何类型实现拾取

新几何类型只需实现局部层，世界层与候选集完全复用。以"自定义图元模型"为例：

```rust
// 1. 实现 AbstractMesh：把数据库数据变成 primitive 迭代器
struct MyPickView<'a> { data: &'a [MyPrimitiveData] }
impl AbstractMesh for MyPickView<'_> {
  type Primitive = Triangle3D;
  fn primitive_count(&self) -> usize { self.data.len() }
  fn primitive_at(&self, primitive_index: usize) -> Option<Self::Primitive> {
    // 从 self.data 构建三角形
    Some(Triangle::new(/* ... */))
  }
}

// 2. 实现 LocalModelPicker：查自己的 payload 外键，查不到返回 None
impl LocalModelPicker for MyPicker {
  fn bounding_enlarge_tolerance(&self, idx: EntityHandle<SceneModelEntity>)
    -> Option<Option<IntersectTolerance>> {
    let _ = self.relation.get(idx)?;
    Some(None) // 三角形不需要容差
  }
  fn ray_query_local_nearest(&self, request: LocalRayQueryRequest)
    -> Option<MeshBufferHitPoint> {
    *self
      .mesh_view(request.idx)?
      .ray_intersect_nearest(request.local_ray, &FaceSide::Double)
  }
  fn ray_query_local_all(&self, request: LocalRayAllQueryRequest) -> Option<()> {
    self.mesh_view(request.internal.idx)?.ray_intersect_all(
      request.internal.local_ray, &FaceSide::Double, request.results,
    );
    Some(())
  }
  fn frustum_query_local(&self, request: LocalFrustumQueryRequest) -> Option<bool> {
    let r = frustum_test_abstract_mesh(&self.mesh_view(request.idx)?, request.policy, |t| {
      frustum_test_tri(request.helper, request.local_frustum, &t, request.policy)
    });
    Some(r)
  }
  fn frustum_query_local_sub_primitives(&self, request: LocalFrustumSubPrimitiveQueryRequest)
    -> Option<()> {
    // 遍历 primitive，frustum_test_tri 命中则 push 索引
    Some(())
  }
}

// 3. 在 use_viewer_scene_model_picker_impl 中把 picker 追加进局部链
//    （见 application/viewer-content/src/pick.rs 的 local_model_pickers vec）
```

若几何是点/线（需要容差），在 `bounding_enlarge_tolerance` 返回 `IntersectTolerance::new(width, ToleranceType::ScreenSpace)` 即可自动获得屏幕空间恒定的拾取手感。

### 模板二：在 viewer 逻辑里做一次射线拾取

```rust
// EventHandling 阶段（参考 widget_bridge.rs 的 inject_picker 模式）
let mut picker = use_viewer_scene_model_picker(cx);
if let Some(picker) = picker.as_mut()
  && let Some((pointer_ctx, scene)) = &picker.pointer_ctx
{
  // 最近命中
  if let Some((hit, model)) =
    picker.pick_model_nearest_all(pointer_ctx.world_ray, *scene)
  {
    // hit.position 是世界空间命中点
  }
  // 全部命中
  let (results, model_ids) = picker.pick_models_list_all(pointer_ctx.world_ray, *scene);
}
```

### 模板三：范围拾取（框选）

```rust
if let Some((frustum, scene)) = create_range_pick_frustum(
  start, end, cx.active_surface_content, &picker.picker_impl, true, 0.,
) {
  let hits = picker.pick_range(scene, &frustum, ObjectTestPolicy::Intersect);
  // hits: Vec<EntityHandle<SceneModelEntity>>
}
```

### 模板四：纯查询（不经 viewer 指针上下文）

直接构造 `SceneRayQuery` 与 `CameraQueryCtx`（像素换算闭包、尺寸、矩阵齐全即可），配合任意的 `SceneModelPicker` 与 `SceneModelIterProvider` 调用 `pick_models_nearest` / `pick_models_all` / `range_pick_models`——geometry-query 本身与 viewer 无依赖，可在任意场景上下文复用（C API 的 `ViewerQueryAPI` 就是这样做的）。

## 延伸阅读

- 拾取的几何基础（Ray3、Frustum、HitPoint3D、OptionalNearest）：[math/geometry/src/dimension3/ray3.rs](../../math/geometry/src/dimension3/ray3.rs)、[dimension3/frustum.rs](../../math/geometry/src/dimension3/frustum.rs)、[intersect_util.rs](../../math/geometry/src/intersect_util.rs)
- 网格求交返回与容差类型：[content/mesh/core/src/feature/intersection.rs](../../content/mesh/core/src/feature/intersection.rs)
- 场景模型数据模型：[scene/core/src/model.rs](../../scene/core/src/model.rs)
- 场景模型包围盒共享计算（供 `sm_world_bounding` / `sm_local_bounding` 使用）：[application/viewer-content/src/bounding.rs](../../application/viewer-content/src/bounding.rs)
