---
name: scene-core-structure
description: >
  rendiation (scene/core) 场景数据模型参考。涵盖所有实体类型
  (SceneEntity、SceneNodeEntity、SceneModelEntity、StandardModelEntity、camera、lights、
  mesh、material、animation、skin)、它们的组件类型、外键关系、
  场景图节点层级、变换传播、SceneWriter API 以及
  StandardModel 渲染模式。底层关系数据库层依赖 database-schema。
  在理解场景模型、添加新的场景实体类型或使用
  SceneWriter/SceneReader 时使用本技能。
metadata:
  version: "1.0"
  updated: "2026-05-17"
  depends: database-schema
---

`scene/core` crate 在 `database` 关系数据库层之上定义了场景数据模型(参见 [[database-schema]])。关键文件:

| 文件 | 用途 |
|------|------|
| [scene/core/src/lib.rs](../../../../../rendiation/scene/core/src/lib.rs) | crate 根,`SceneEntity`、`register_scene_core_data_model()` |
| [scene/core/src/node.rs](../../../../../rendiation/scene/core/src/node.rs) | 场景图节点与世界变换推导 |
| [scene/core/src/model.rs](../../../../../rendiation/scene/core/src/model.rs) | `SceneModelEntity`、`StandardModelEntity`,模型到场景的接线 |
| [scene/core/src/mesh.rs](../../../../../rendiation/scene/core/src/mesh.rs) | `AttributesMeshEntity`、顶点缓冲区关系、实例化网格 |
| [scene/core/src/material.rs](../../../../../rendiation/scene/core/src/material.rs) | Unlit、PBR specular-glossiness、PBR metallic-roughness 材质 |
| [scene/core/src/camera.rs](../../../../../rendiation/scene/core/src/camera.rs) | 带透视/正交/自定义投影的 `SceneCameraEntity` |
| [scene/core/src/light.rs](../../../../../rendiation/scene/core/src/light.rs) | 点光源、聚光灯、平行光 |
| [scene/core/src/texture.rs](../../../../../rendiation/scene/core/src/texture.rs) | 2D 纹理、立方体贴图、采样器 |
| [scene/core/src/buffer.rs](../../../../../rendiation/scene/core/src/buffer.rs) | 原始 GPU 缓冲区存储 |
| [scene/core/src/animation.rs](../../../../../rendiation/scene/core/src/animation.rs) | 动画资产与通道 |
| [scene/core/src/skin.rs](../../../../../rendiation/scene/core/src/skin.rs) | 蒙皮与关节 |
| [scene/core/src/writer.rs](../../../../../rendiation/scene/core/src/writer.rs) | `SceneWriter` — 统一的写入接口 |
| [scene/core/src/reader.rs](../../../../../rendiation/scene/core/src/reader.rs) | `SceneReader` — 统一的读取接口 |

所有实体/组件声明都位于 crate 根的 `register_scene_core_data_model()` 中。

## 实体关系总览

```
SceneEntity
 ├─ SceneHDRxEnvBackgroundCubeMap ──────────→ SceneTextureCubeEntity
 │
 ├─ [light refs] ──→ Point/Spot/DirectionalLightEntity
 ├─ [model refs] ──→ SceneModelEntity
 ├─ [camera refs] ─→ SceneCameraEntity
 └─ [animation refs] → SceneAnimationEntity

SceneNodeEntity
 ├─ SceneNodeParentIdx ─────────────────────→ SceneNodeEntity (optional parent)
 │
 ├─ [model refs] ──→ SceneModelEntity (via SceneModelRefNode)
 ├─ [camera refs] ─→ SceneCameraEntity (via SceneCameraNode)
 ├─ [light refs] ──→ Point/Spot/DirectionalLightEntity (via *RefNode)
 ├─ [skin refs] ───→ SceneSkinEntity (via SceneSkinRoot)
 └─ [joint refs] ──→ SceneJointEntity (via SceneJointRefNode)

SceneModelEntity
 ├─ SceneModelBelongsToScene ───────────────→ SceneEntity
 ├─ SceneModelRefNode ──────────────────────→ SceneNodeEntity
 └─ SceneModelStdModelRenderPayload ────────→ StandardModelEntity

StandardModelEntity
 ├─ StandardModelRefAttributesMeshEntity ───→ AttributesMeshEntity
 ├─ StandardModelRef{Unlit,PbrSG,PbrMR}Material → material entity
 └─ StandardModelRefSkin ───────────────────→ SceneSkinEntity (optional)
```

核心原则:**SceneEntity** 是容器。**SceneNodeEntity** 提供位置(变换)。**SceneModelEntity** 将 **StandardModelEntity**(渲染什么:网格 + 材质)桥接到特定场景中的特定节点(放在哪里)。

## 实体类型及其组件

### SceneEntity — 顶层场景容器

```
declare_entity!(SceneEntity)
```

| 组件 | 类型 | 用途 |
|-----------|------|------|
| `SceneSolidBackground` | `Option<Vec3<f32>>` | 纯色背景 |
| `SceneGradientBackgroundInfo` | `Option<SceneGradientBackgroundParam>` | 渐变背景 |
| `SceneHDRxEnvBackgroundInfo` | `Option<SceneHDRxEnvBackgroundParameter>` | HDR 环境(强度 + 变换) |
| `SceneHDRxEnvBackgroundCubeMap` | 外键 → `SceneTextureCubeEntity` | HDR 环境立方体贴图 |

### SceneNodeEntity — 场景图节点

```
declare_entity!(SceneNodeEntity)
```

| 组件 | 类型 | 用途 |
|-----------|------|------|
| `SceneNodeLocalMatrixComponent` | `Mat4<f64>` | 局部变换(默认为单位矩阵,f64 保证精度) |
| `SceneNodeVisibleComponent` | `bool` | 可见性标志(默认为 true) |
| `SceneNodeParentIdx` | 外键 → `SceneNodeEntity` | 可选父节点,构成场景层级 |

**世界矩阵**:由 `GlobalNodeDerive` 派生 — 将 `parent_world * local` 从根传播到叶节点。净可见性类似传播(`visible && all_parents_visible`)。

### SceneModelEntity — 渲染负载与场景之间的桥接

```
declare_entity!(SceneModelEntity)
```

| 组件 | 类型 | 用途 |
|-----------|------|------|
| `SceneModelBelongsToScene` | 外键 → `SceneEntity` | 该模型属于哪个场景 |
| `SceneModelRefNode` | 外键 → `SceneNodeEntity` | 哪个节点定位该模型 |
| `SceneModelStdModelRenderPayload` | 外键 → `StandardModelEntity` | 渲染什么 |

### StandardModelEntity — 可渲染负载

```
declare_entity!(StandardModelEntity)
```

| 组件 | 类型 | 用途 |
|-----------|------|------|
| `StandardModelRefAttributesMeshEntity` | 外键 → `AttributesMeshEntity` | 要渲染的网格 |
| `StandardModelRefUnlitMaterial` | 外键 → `UnlitMaterialEntity` | Unlit 材质(同一时刻仅一种材质类型) |
| `StandardModelRefPbrSGMaterial` | 外键 → `PbrSGMaterialEntity` | PBR specular-glossiness 材质 |
| `StandardModelRefPbrMRMaterial` | 外键 → `PbrMRMaterialEntity` | PBR metallic-roughness 材质 |
| `StandardModelRefSkin` | 外键 → `SceneSkinEntity` | 可选蒙皮 |
| `StandardModelRasterizationOverride` | `Option<RasterizationStates>` | 可选光栅化状态覆盖 |

### AttributesMeshEntity — GPU 网格数据

```
declare_entity!(AttributesMeshEntity)
```

| 组件 | 类型 | 用途 |
|-----------|------|------|
| `AttributesMeshEntityTopology` | `MeshPrimitiveTopology` | 三角形列表、线段列表等 |
| `AttributesMeshBoundingConfig` | `BoundingConfig` | 计算或用户定义的包围盒 |
| `AttributeIndexRef` | → `BufferEntity`(经由 `SceneBufferView` 组件) | 索引缓冲区 |

顶点缓冲区通过一个独立的关系实体存储:

```
declare_entity!(AttributesMeshEntityVertexBufferRelation)
```

| 组件 | 类型 | 用途 |
|-----------|------|------|
| `AttributesMeshEntityVertexBufferSemantic` | `AttributeSemantic` | 位置、法线、UV 等 |
| 外键 → `AttributesMeshEntity` | 外键 | 该顶点缓冲区属于哪个网格 |
| `AttributeVertexRef` | → `BufferEntity`(经由 `SceneBufferView` 组件) | 顶点缓冲区数据 |

### 相机

```
declare_entity!(SceneCameraEntity)
```

| 组件 | 类型 | 用途 |
|-----------|------|------|
| `SceneCameraNode` | 外键 → `SceneNodeEntity` | 相机变换(取自节点世界矩阵) |
| `SceneCameraPerspective` | `Option<PerspectiveProjection<f32>>` | 透视投影 |
| `SceneCameraOrthographic` | `Option<OrthographicProjection<f32>>` | 正交投影 |
| `SceneCameraProjectionCustomOverride` | `Option<Mat4<f32>>` | 自定义投影覆盖 |

### 灯光

```
declare_entity!(PointLightEntity)
declare_entity!(SpotLightEntity)
declare_entity!(DirectionalLightEntity)
```

所有灯光共享同一种模式:外键 → `SceneEntity`(属于场景)+ 外键 → `SceneNodeEntity`(位置/方向来自节点变换)。

| 类型 | 专属组件 |
|------|-------------------|
| PointLight | `PointLightIntensity: Vec3<f32>` (cd),`PointLightCutOffDistance: f32` |
| SpotLight | `SpotLightIntensity: Vec3<f32>`, `SpotLightCutOffDistance: f32`, `SpotLightHalfConeAngle: f32`, `SpotLightHalfPenumbraAngle: f32` |
| DirectionalLight | `DirectionalLightIlluminance: Vec3<f32>` (lux) |

### 材质

```
declare_entity!(UnlitMaterialEntity)       // color + optional alpha texture
declare_entity!(PbrSGMaterialEntity)       // specular-glossiness PBR
declare_entity!(PbrMRMaterialEntity)       // metallic-roughness PBR
```

所有材质实体都有纹理槽位,通过 `TextureWithSamplingForeignKeys` 引用 `SceneTexture2dEntity` + `SceneSamplerEntity` 对。

| 材质 | 关键组件 | 默认值 |
|----------|---------------|----------|
| Unlit | `UnlitMaterialColorComponent: Vec4<f32>` | (1,1,1,1) |
| PBR SG | `PbrSGMaterialAlbedoComponent: Vec3<f32>`, `PbrSGMaterialSpecularComponent: Vec3<f32>`, `PbrSGMaterialGlossinessComponent: f32` | (1,1,1), (0,0,0), 0.5 |
| PBR MR | `PbrMRMaterialBaseColorComponent: Vec3<f32>`, `PbrMRMaterialMetallicComponent: f32`, `PbrMRMaterialRoughnessComponent: f32` | (1,1,1), 0.0, 0.5 |

所有材质都有 `AlphaConfig`(模式/混合/裁切)以及 `EmissiveComponent` + 自发光纹理。

### 其他实体

| 实体 | 用途 |
|--------|--------|
| `SceneTexture2dEntity` | 2D 纹理(直接数据或 URI) |
| `SceneTextureCubeEntity` | 立方体贴图(6 个外键面 → `SceneTexture2dEntity`) |
| `SceneSamplerEntity` | 纹理采样器配置 |
| `BufferEntity` | 原始 GPU 缓冲区(`Arc<Vec<u8>>`) |
| `InstanceMeshInstanceEntity` | 实例化网格(世界矩阵 + 对 `AttributesMeshEntity` 的引用) |
| `SceneAnimationEntity` | 动画资产(外键 → `SceneEntity`) |
| `SceneAnimationChannelEntity` | 动画通道(目标为 `SceneNodeEntity`,存储插值与关键帧缓冲区) |
| `SceneSkinEntity` | 蒙皮定义(根节点外键) |
| `SceneJointEntity` | 关节(外键 → 节点 + 蒙皮,存储关节索引与逆绑定矩阵) |

## SceneWriter API

定义于 [scene/core/src/writer.rs](../../../../../rendiation/scene/core/src/writer.rs)。通过 `SceneWriter::from_global()` 构造。

### 实体写入器(公共字段)

每种实体类型都有一个专属的写入器字段:

```rust
writer.node_writer           // TableWriter<SceneNodeEntity>
writer.std_model_writer      // TableWriter<StandardModelEntity>
writer.model_writer          // TableWriter<SceneModelEntity>
writer.mesh_writer           // AttributesMeshEntityFromAttributesMeshWriter
writer.pbr_sg_mat_writer     // TableWriter<PbrSGMaterialEntity>
writer.pbr_mr_mat_writer     // TableWriter<PbrMRMaterialEntity>
writer.unlit_mat_writer      // TableWriter<UnlitMaterialEntity>
writer.camera_writer         // TableWriter<SceneCameraEntity>
writer.directional_light_writer
writer.point_light_writer
writer.spot_light_writer
writer.scene_writer          // TableWriter<SceneEntity>
writer.tex_writer            // TableWriter<SceneTexture2dEntity>
writer.cube_writer           // TableWriter<SceneTextureCubeEntity>
writer.sampler_writer        // TableWriter<SceneSamplerEntity>
writer.buffer_writer         // TableWriter<BufferEntity>
writer.animation             // TableWriter<SceneAnimationEntity>
writer.animation_channel     // TableWriter<SceneAnimationChannelEntity>
writer.skin_writer           // TableWriter<SceneSkinEntity>
writer.joint_writer          // TableWriter<SceneJointEntity>
```

### 关键方法

注意:当前版本没有"目标场景"概念 — 场景句柄 `EntityHandle<SceneEntity>` 由调用方显式持有,并作为参数传入所有与场景相关的方法(早期版本的 `expect_target_scene()` / `replace_target_scene()` 已移除)。

| 方法 | 用途 |
|--------|--------|
| `create_root_child()` → `EntityHandle<SceneNodeEntity>` | 创建无父节点的节点 |
| `create_child(parent)` → `EntityHandle<SceneNodeEntity>` | 创建以 `parent` 为父节点的节点 |
| `set_local_matrix(node, Mat4<f64>)` | 设置节点的局部变换 |
| `get_local_mat(node)` → `Option<Mat4<f64>>` | 读取节点的局部变换 |
| `create_scene_model(material, mesh, node, scene)` → `(EntityHandle<StandardModelEntity>, EntityHandle<SceneModelEntity>)` | 创建 StandardModel + SceneModel,并接线到节点与场景 |
| `write_attribute_mesh(mesh)` → `AttributesMeshEntities` | 写入 `AttributesMesh` 数据,返回句柄 |
| `write_attribute_mesh_data_uri(mesh, buffer_source)` → `AttributesMeshEntities` | 以数据 URI 方式写入网格(由资产系统追踪) |
| `set_solid_background(solid: Vec3<f32>, scene)` | 设置纯色背景 |
| `set_gradient_background(gradient, scene)` | 设置渐变背景 |
| `set_hdr_env_background(cube_map, intensity, transform, scene)` | 设置 HDR 环境 |
| `texture_sample_pair_writer()` | 创建纹理 + 采样器对的辅助工具 |

## StandardModel 模式

创建可渲染对象的常规路径:

- 创建 AttributesMesh(GPU 网格数据)
- 创建材质实体(Unlit/PbrSG/PbrMR)
- 创建 SceneNodeEntity(通过变换定位)
- 调用 `SceneWriter::create_scene_model(material, mesh, node, scene)`
  - 内部创建 StandardModelEntity + SceneModelEntity 并完成接线

`create_scene_model` 接受 `SceneMaterialDataView` 枚举:
```rust
SceneMaterialDataView::PbrSGMaterial(handle)
SceneMaterialDataView::PbrMRMaterial(handle)
SceneMaterialDataView::UnlitMaterial(handle)
SceneMaterialDataView::Other  // 哨兵值
```

读取时(`SceneReader::read_std_model`),材质按优先级解析:先 PBR MR,再 PBR SG,最后 Unlit。

## 场景图与变换

- **局部变换**:每个 `SceneNodeEntity` 上的 `Mat4<f64>`,经 `SceneNodeLocalMatrixComponent` 存储
- **世界变换**:由 `GlobalNodeDerive` 派生 — `node_world_mat(this, parent) = parent_world * local`(根节点则为 `local`)
- **净可见性**:派生值 — `node_net_visible(this, parent) = this_visible && parent_net_visible`
- **模型世界矩阵**:将节点世界矩阵与 `SceneModelRefNode` 反向引用连接后派生
- **相机变换**:节点世界矩阵 + 投影 → `CameraTransform`(含 view、projection、VP 及逆矩阵)

## 注册

场景模型的全部实体、组件与外键都在 `register_scene_core_data_model()` ([scene/core/src/lib.rs](../../../../../rendiation/scene/core/src/lib.rs#L43)) 中注册。该函数必须在应用初始化期间、写入任何场景数据之前调用。
