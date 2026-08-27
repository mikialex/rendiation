# 光照系统指南（content/lighting 与 viewer 的光照装配）

本文梳理 rendiation 的 GPU 光照系统：从场景数据库里的灯光实体（方向光/聚光/点光/区域光/IBL 环境光）到屏幕上最终像素的完整链路。覆盖 [content/lighting](../../content/lighting) 下的几个 crate（`rendiation_lighting_gpu_system` 的 trait 体系、`rendiation_lighting_shadow_map` 的阴影图集与 PCF/VSM 过滤、`rendiation_lighting_punctual` 的着色器侧点光源、`rendiation_lighting_transport` 的表面着色模型、`rendiation_lighting_ibl` 的环境光预过滤、`rendiation_lighting_ltc` 的区域光 LTC），以及 viewer 应用层把它们装进帧里的 [rendering/lighting](../../application/viewer-content/src/rendering/lighting/mod.rs) 模块——`use_lighting`（常驻维护）、`LightSystem::prepare`（每帧组装）、`use_render_lighting_scene_content`（前向/延迟光照 pass）。

这条链路是 viewer 帧流水线的「光照语义」所在：材质侧注册的通道（`ColorChannel` / `EmissiveChannel` …）与标签（`LightableSurfaceTag` / `PbrMRMaterialTag` …）在这里被消费成屏幕颜色。本文与 [viewer-content-frame-pipeline-guide.md](viewer-content-frame-pipeline-guide.md) 的「光照系统接入」一节互补——那篇是帧组织视角（谁调用谁），本篇是子系统内部视角（光照如何被准备、打包、在着色器里算出来）。

## 前置阅读

光照系统横跨场景数据模型、GPU 组件模型、着色器 EDSL 与帧组装，建议按序了解：

| 文档 | 内容 |
| --- | --- |
| [skill-translation/scene-core-structure-zh.md](skill-translation/scene-core-structure-zh.md) | 场景实体类型、SceneWriter、外键语义（光照实体挂在哪个表上） |
| [skill-translation/shader-edsl-core-zh.md](skill-translation/shader-edsl-core-zh.md) | Node&lt;T&gt;、着色器结构体、内存布局（光照 uniform 的宿主侧/着色器侧对应） |
| [skill-translation/shader-edsl-graphics-zh.md](skill-translation/shader-edsl-graphics-zh.md) | 语义注册（register/query/contains_type_tag）、fragment 阶段（光照结果的语义接线） |
| [skill-translation/shader-edsl-binding-and-typed-container-zh.md](skill-translation/shader-edsl-binding-and-typed-container-zh.md) | bind_by / bind、UniformBufferDataView（光照 uniform 的绑定契约） |
| [skill-translation/fundamental-gpu-component-model-zh.md](skill-translation/fundamental-gpu-component-model-zh.md) | RenderComponent / ShaderHashProvider / RenderArray（光照组件如何参与管线哈希） |
| [skill-translation/frame-pass-assemble-zh.md](skill-translation/frame-pass-assemble-zh.md) | pass() / render_ctx() / keyed_scope（阴影贴图与光照 pass 的帧内语法） |
| [skill-translation/query-system-zh.md](skill-translation/query-system-zh.md) | 增量查询、use_changes、稀疏写（光照 uniform 的增量维护底座） |
| [skill-translation/viewer-scene-building-zh.md](skill-translation/viewer-scene-building-zh.md) | 灯光实体的创建配方（用户视角，见其「Lights」一节） |
| [viewer-content-frame-pipeline-guide.md](viewer-content-frame-pipeline-guide.md) | 光照在 viewer 帧内的接入位置（本文的帧组织视角，含 Update/CreateRender 两阶段） |
| [gles-material-host-render-guide.md](gles-material-host-render-guide.md) | 材质侧如何注册通道与标签（光照 pass 的消费输入）；文末有 GLES 光照 uniform 数组的线索 |
| [material-indirect-render-guide.md](material-indirect-render-guide.md) | 间接材质路径的通道/标签注册（两后端共享的光照消费输入） |
| [webgpu-hook-utils-guide.md](webgpu-hook-utils-guide.md) | use_uniform_buffers / UniformBufferCollection / update_uniforms（光照 uniform 的增量写基建） |

## 模式概览

整条光照链路可以浓缩为四句话：

- **灯光是数据库里的实体，渲染侧按场景聚合。** 每类灯光实体（方向/聚光/点光/区域光）都有「灯光 → 场景」与「灯光 → 节点」两个外键，渲染侧按 `light_ref_scene` 外键把灯光聚合成「每场景一个」的 uniform 数组（`PerSceneLightUniformArray`，数组长度上限 `LIGHT_LIST_LEN = 8`，[gpu-gles/src/light/mod.rs:117](../../scene/rendering/gpu-gles/src/light/mod.rs#L117)）；光照与阴影共享同一份「灯光 → 数组下标」分配映射（`allocation_info`），所以着色器里灯光的数组下标就是阴影信息的数组下标。
- **trait 体系把「算光照」切成四层。** `LightSystemSceneProvider`（按场景产出一份光照计算组件）→ `LightingComputeComponent`（绑定资源、产出调用）→ `LightingComputeInvocation`（对着色器几何上下文与表面着色模型计算光照）→ `LightableSurfaceShading`（材质侧的 BRDF）。`LightingComputeComponentAsRenderComponent` 把后三层接进 `RenderComponent`，与 tonemap、emissive 加成、LDR 输出合成一个组件，插进材质渲染的 dispatcher。
- **阴影是「批渲染 + 图集打包 + 过滤」三段。** `LightSystem::prepare` 逐光源提取不透明批（阴影也走批提取器）画进共享的 2048³ 深度图集（`MultiLayerTexturePacker` 按灯光 ID 打包 2D 区域到 2D-array 纹理的层）；PCF/VSM 两种 computer 各自把图集作为采样源；`ShadowMapAddressInfo`（层 + 偏移 + 尺寸）作为 uniform 随每个光源下发。
- **通道与标签是光照的「接线图」。** 材质在着色器里注册 `ColorChannel` / `RoughnessChannel` / `MetallicChannel` / `EmissiveChannel` 等通道和 `LightableSurfaceTag` 标签；光照组件在 `post_build` 里检测标签，把 `HDRLightResult`（光照+emissive）→ tonemap → `LDRLightResult` → `DefaultDisplay` 的链在片段阶段逐级 query/register 完成。

## 核心概念

| 概念 | 定义位置 | 说明 |
| --- | --- | --- |
| `LightSystem` | [viewer lighting/mod.rs:205](../../application/viewer-content/src/rendering/lighting/mod.rs#L205) | 光照常驻配置：表面模型（Pbr/SimplePhong）、tonemap、阴影过滤（PCF/VSM）、延迟材质注册表、光照技术（Forward/DeferLighting） |
| `use_lighting` | [lighting/mod.rs:15](../../application/viewer-content/src/rendering/lighting/mod.rs#L15) | Update/CreateRender 两阶段执行：四类光源 uniform + IBL + scene id，产出 `LightingRenderingCxPrepareCtx` |
| `LightSystem::prepare` | [lighting/mod.rs:66](../../application/viewer-content/src/rendering/lighting/mod.rs#L66) | 帧内组装点：绘制全部阴影贴图、组装 `SceneLightSystem`，产出 `LightingRenderingCx` |
| `SceneLightSystem` | [lighting/mod.rs:422](../../application/viewer-content/src/rendering/lighting/mod.rs#L422) | 每帧光照上下文：scene id + 配置 + `LightSystemSceneProvider`；`get_scene_forward_lighting_component` 组装前向光照 `RenderComponent` |
| `LightingRenderingCx` | [light_pass/mod.rs:13](../../application/viewer-content/src/rendering/lighting/light_pass/mod.rs#L13) | `SceneLightSystem` + tonemap + 延迟材质注册表 + `LightingTechniqueKind` |
| `LightingComputeComponent` | [lighting-system/src/lib.rs:10](../../content/lighting/gpu-system/lighting-system/src/lib.rs#L10) | 一类光源的光照计算组件：着色器侧绑定 + pass 侧绑定 + 管线哈希 |
| `LightingComputeInvocation` | [lighting-system/src/lib.rs:139](../../content/lighting/gpu-system/lighting-system/src/lib.rs#L139) | 着色器侧调用：对给定表面着色模型与几何上下文算出一份 `ShaderLightingResult` |
| `LightSystemSceneProvider` | [gpu-base/src/light.rs:5](../../scene/rendering/gpu-base/src/light.rs#L5) | 按 (scene, camera) 产出 `LightingComputeComponent`（camera 供级联阴影使用） |
| `LightableSurfaceTag` | [lighting-system/src/lib.rs:86](../../content/lighting/gpu-system/lighting-system/src/lib.rs#L86) | 片段阶段标签：材质可被光照；检测不到就跳过光照计算 |
| `AbstractShadowMapGPUData` | [shadow-map/src/lib.rs:66](../../content/lighting/gpu-system/shadow-map/src/lib.rs#L66) | 阴影图集抽象：重建、清空、逐光源更新、产出采样 computer |
| `AbstractShadowComputer` | [shadow-map/src/lib.rs:35](../../content/lighting/gpu-system/shadow-map/src/lib.rs#L35) | 阴影过滤实现（PCF / VSM）的着色器侧抽象 |
| `ShadowMapAddressInfo` | [shadow-map/src/lib.rs:127](../../content/lighting/gpu-system/shadow-map/src/lib.rs#L127) | 一个阴影区域在图集中的位置：层 + 尺寸 + 偏移（std140，随光源 uniform 下发） |
| `MultiLayerTexturePackerConfig` | [packer pack_2d_to_3d/mod.rs:137](../../content/texture/packer/src/pack_2d_to_3d/mod.rs#L137) | 图集打包配置：init_size（2 层起步，webgl 兼容）与 max_size（2048³、3 层） |
| `PerSceneLightUniformArray` | [gpu-gles/src/light/mod.rs:14](../../scene/rendering/gpu-gles/src/light/mod.rs#L14) | 按场景聚合的灯光 uniform 数组 + 灯光分配映射（两后端共用） |
| `FrameGeneralMaterialBuffer` | [light_pass/defer_protocol.rs:7](../../application/viewer-content/src/rendering/lighting/light_pass/defer_protocol.rs#L7) | 延迟光照的材质缓冲：材质类型 id + 三个编码通道 |

## 场景侧光照数据模型

### 灯光实体（scene/core）

四类灯光实体都定义在 [scene/core/src/light.rs](../../scene/core/src/light.rs)，结构完全同构：`declare_entity` + 强度组件 + 两个外键（`XxxRefScene` 指向场景、`XxxRefNode` 指向节点）+ `Enabled` 组件。方向光多一个 `DirectionalLightFollowCamera`（跟随相机，[light.rs:166](../../scene/core/src/light.rs#L166)）。

| 实体 | 强度组件（单位） | 几何来源 | 特有参数 |
| --- | --- | --- | --- |
| `DirectionalLightEntity` | `DirectionalLightIlluminance`（lx，lux） | 节点的 world 矩阵 forward 反方向 | `DirectionalLightFollowCamera` |
| `SpotLightEntity` | `SpotLightIntensity`（cd，candela） | 节点 world 位置 + forward | `SpotLightCutOffDistance` / `SpotLightHalfConeAngle` / `SpotLightHalfPenumbraAngle` |
| `PointLightEntity` | `PointLightIntensity`（cd） | 节点 world 位置 | `PointLightCutOffDistance` |
| `AreaLightEntity`（extension） | `AreaLightIntensity`（无单位） | 节点 world 矩阵展开的四顶点 | `AreaLightSize`（米）/ `AreaLightIsRound` / `AreaLightIsDoubleSide`，定义在 [extension/area-lighting/src/lib.rs:27](../../extension/area-lighting/src/lib.rs#L27) |

用户创建灯光的写法见 [viewer-scene-building-zh.md](skill-translation/viewer-scene-building-zh.md) 的「Lights」一节：`DirectionalLightDataView { illuminance, node, scene }.write(&mut writer.directional_light_writer)`（[light.rs:139](../../scene/core/src/light.rs#L139)），其余三类同构。所有 DataView 都只写外键与参数，不写位置——位置/方向来自节点世界矩阵。

### 阴影配置组件（viewer 侧）

阴影参数不是灯光实体自带的，而是 viewer 应用层在 [rendering/lighting/shadow.rs](../../application/viewer-content/src/rendering/lighting/shadow.rs) 里用「标签实体关联组件」模式附加的：`BasicShadowMapResolutionOf<T>`（默认 256×256）、`BasicShadowMapBiasOf<T>`（`ShadowBiasConfig { bias, normal_bias }`）、`BasicShadowMapEnabledOf<T>`（默认开）三个组件通过 `register_basic_shadow_map_for_light` 注册到三类灯光表上（[shadow.rs:62](../../application/viewer-content/src/rendering/lighting/shadow.rs#L62)）；方向光还多一个 `DirectionLightShadowBound`（正交投影范围，米，缺省 `DEFAULT_DIR_PROJ` ±20 米、near 0 far 1000，[light_source/directional.rs:6](../../application/viewer-content/src/rendering/lighting/light_source/directional.rs#L6)）。注册入口 `register_light_shadow_config` 在 [viewer-content/src/lib.rs:128](../../application/viewer-content/src/lib.rs#L128) 调用。

### 环境背景（IBL 输入）

`SceneEntity` 上挂着 `SceneHDRxEnvBackgroundInfo`（`transform` + `intensity`，[scene/core/src/lib.rs:75](../../scene/core/src/lib.rs#L75)）与 `SceneHDRxEnvBackgroundCubeMap` 外键（指向 `SceneTextureCubeEntity`）——IBL 就是「每场景一个环境立方体贴图」。

## 光照系统的整体组织

### 常驻配置：LightSystem

`LightSystem::new`（[lighting/mod.rs:224](../../application/viewer-content/src/rendering/lighting/mod.rs#L224)）在 `Viewer3dRenderingCtx::new` 时创建一次，是 viewer 中光照的「配置文件」：

- `lighting_surface_ty`：`ViewerLightSurfaceType::Pbr / SimplePhong` 二选一（[mod.rs:179](../../application/viewer-content/src/rendering/lighting/mod.rs#L179)），`create_impl` 产出 `Box<dyn LightableSurfaceProvider>`。
- `material_defer_lighting_supports`：`DeferLightingMaterialRegistry`，注册了 Pbr/Unlit/Phong 三套 encode/decode（延迟光照用，见「延迟光照」一节）。
- `opaque_scene_content_lighting_technique`：`LightingTechniqueKind`（Forward 默认 / DeferLighting）。
- 阴影相关：`enable_shadow`、`filter_ty`（PCF/VSM）、`pcf_config`、`vsm_config`、`bias_behavior`、级联开关 `use_cascade_shadowmap_for_directional_lights` 与 `cascade_shadow_split_linear_log_blend_ratio`。

`LightSystem::egui`（[mod.rs:247](../../application/viewer-content/src/rendering/lighting/mod.rs#L247)）把全部配置暴露成调试面板——运行时改的都是这份配置，下一帧生效。

### Update/CreateRender 阶段：use_lighting

[lighting/mod.rs:15](../../application/viewer-content/src/rendering/lighting/mod.rs#L15) 的 `use_lighting` 在 `use_viewer_scene_renderer` 末尾调用（两后端共用），五个组成部分：

```rust
let dir_lights  = use_directional_light_uniform(cx, &config, viewports, lighting_sys, ndc);
let spot_lights = use_scene_spot_light_uniform(cx, &config, lighting_sys, ndc);
let point_lights = use_scene_point_light_uniform(cx, &config, lighting_sys, ndc);
let area_lights = use_area_light_uniform(cx);
let ibl = use_ibl(cx);
let scene_ids = use_scene_id_provider(cx);
cx.when_render(|| LightingRenderingCxPrepareCtx { ... })
```

- 图集配置在此创建（[mod.rs:21](../../application/viewer-content/src/rendering/lighting/mod.rs#L21)）：`init_size` 2048² × 2 层（注释说明 2 层起步是为了 webgl），`max_size` 2048² × 3 层；注释明确当前图集打包不带 padding，PCF 采样可能跨区域采样到邻居区域（已知限制）。
- `when_render` 包装保证 CreateRender 阶段才产出 `LightingRenderingCxPrepareCtx`——Update 阶段各 `use_*` 内部只有增量维护，`unwrap()` 只发生在渲染阶段。
- `use_scene_id_provider`（[gpu-base/src/scene_id.rs:6](../../scene/rendering/gpu-base/src/scene_id.rs#L6)）维护「场景实体 → 分配索引」uniform，供着色器按场景取光源。

`LightingRenderingCxPrepareCtx`（[mod.rs:55](../../application/viewer-content/src/rendering/lighting/mod.rs#L55)）是常驻维护与渲染实例之间的传递物：方向/聚光/点光各持一份「光源 preparer + 阴影图集句柄」，区域光持 LTC LUT + uniform，IBL 持预过滤结果。

### render 阶段：LightSystem::prepare

[lighting/mod.rs:66](../../application/viewer-content/src/rendering/lighting/mod.rs#L66) 的 `prepare` 在每帧 `Viewer3dRenderingCtx::render` 开头调用（[frame_all.rs:621](../../application/viewer-content/src/rendering/frame_all.rs#L621)），分两步：

**第一步：绘制全部阴影贴图。** 帧内定义 `content` 闭包（[mod.rs:86](../../application/viewer-content/src/rendering/lighting/mod.rs#L86)）做一次「用阴影相机画场景到图集区域」：

```rust
let camera_uniform = UniformBufferDataView::create(...);      // 阴影相机（投影 + 世界矩阵）
let depth = ();                                               // 没有颜色通道，空 dispatcher
let batch = extractor.extract_scene_batch(scene_id, key, renderer);  // 阴影也走批提取器，只取不透明批
frame_ctx.keyed_scope(&shadow_id, |frame_ctx| {               // 每个阴影一个 scope 缓存（跨帧复用）
  current_lod_camera.set(Some(LODCameraInfo { camera: camera_uniform, view_resolution, lod_error_threshold }));
  let content = renderer.use_make_scene_batch_pass_content(batch, frame_ctx);
  map_desc.render_ctx(frame_ctx).by(&mut content.as_pass_content(&camera, &depth));  // 画进图集区域
});
shadow_id += 1;                                               // 阴影序号递增 = scope 身份
```

要点：阴影相机 uniform 直接在这里现场创建；`LODCameraInfo` 让 LOD 属性网格在阴影相机下也按图集分辨率选级；`map_desc.render_ctx`（[shadow-map/src/lib.rs:93](../../content/lighting/gpu-system/shadow-map/src/lib.rs#L93)）在进入 pass 前 `set_viewport` 到图集区域。`scene_id` 由各 preparer 通过 `read_global_db_foreign_key()` 读「灯光 → 场景」外键补上——批提取需要场景 id。

三类点光源的 preparer 各自驱动这段闭包：`dir_lights.update_shadow_maps(...)` / `spot_lights` / `point_lights`（[mod.rs:142-152](../../application/viewer-content/src/rendering/lighting/mod.rs#L142)）。

**第二步：组装光照组件组。** 五类光源被包成 `LightingComputeComponentGroupProvider`（[gpu-base/src/light.rs:14](../../scene/rendering/gpu-base/src/light.rs#L14)，实现 `LightSystemSceneProvider` 的 Vec 聚合），与 scene id、系统配置合成 `SceneLightSystem`，产出 `LightingRenderingCx`。之后每视口的 `use_render_lighting_scene_content` 都从 `LightingRenderingCx` 取数。

### SceneLightSystem 与 get_scene_forward_lighting_component

[lighting/mod.rs:442](../../application/viewer-content/src/rendering/lighting/mod.rs#L442) 的 `get_scene_lighting_component` 是着色器侧「光照语义」的组装点，返回一个 `RenderVec`（组件数组）：

```rust
light.push(LDROutput)                        // 片段阶段：LightableSurfaceTag → LDRLightResult → DefaultDisplay
light.push(&system.tonemap)                  // HDRLightResult → LDRLightResult（exposure 乘算）
light.push(&ForwardLightingEmissiveAdd)      // HDRLightResult += EmissiveChannel（注释：只允许单次绘制，否则 emissive 重复加）
light.push(LightingComputeComponentAsRenderComponent { scene_id, geometry_constructor, surface_constructor, lighting })
```

`get_scene_forward_lighting_component`（[mod.rs:429](../../application/viewer-content/src/rendering/lighting/mod.rs#L429)）是它的便捷版：几何用 `DirectGeometryProvider`（直接从顶点插值重建几何上下文），表面用 `LightSystem.lighting_surface_ty`（Pbr 或 SimplePhong）。延迟光照路径则替换这两个参数（见「延迟光照」）。`enable_channel_debugger` 开启时用 `ScreenChannelDebugger`（[debug_channels.rs:8](../../application/viewer-content/src/rendering/lighting/debug_channels.rs#L8)）替代 `LDROutput`，把通道按屏宽切片可视化。

## trait 抽象体系与下游实现

### 四层光照计算 trait

[lighting-system/src/lib.rs](../../content/lighting/gpu-system/lighting-system/src/lib.rs) 定义光照计算的核心抽象：

| trait | 定义 | 职责 |
| --- | --- | --- |
| `LightSystemSceneProvider` | [gpu-base/src/light.rs:5](../../scene/rendering/gpu-base/src/light.rs#L5) | 按 (scene, camera) 产出 `Option<Box<dyn LightingComputeComponent>>`；camera 参数供级联阴影的每相机数据 |
| `LightingComputeComponent` | [lighting-system/src/lib.rs:10](../../content/lighting/gpu-system/lighting-system/src/lib.rs#L10) | `build_light_compute_invocation(binding, scene_id)` 产出着色器侧调用；`setup_pass` 做 pass 侧绑定；`ShaderHashProvider` 进管线哈希 |
| `LightingComputeInvocation` | [lighting-system/src/lib.rs:139](../../content/lighting/gpu-system/lighting-system/src/lib.rs#L139) | `compute_lights(shading, geom_ctx) -> ENode<ShaderLightingResult>`，对着 `LightableSurfaceShading` 算光照 |
| `GeometryCtxProvider` | [lighting-system/src/lib.rs:19](../../content/lighting/gpu-system/lighting-system/src/lib.rs#L19) | `construct_ctx(builder) -> ENode<ShaderLightingGeometricCtx>`：前向用顶点插值，延迟用 g-buffer 重建 |
| `LightableSurfaceProvider` | [lighting-system/src/lib.rs:26](../../content/lighting/gpu-system/lighting-system/src/lib.rs#L26) | `construct_shading(builder, binding) -> Box<dyn LightableSurfaceShading>`：从通道构造表面着色模型 |
| `LightableSurfaceShading` | [transport/src/surface/mod.rs:148](../../content/lighting/transport/src/surface/mod.rs#L148) | `compute_lighting_by_incident(direct_light, geom_ctx) -> ENode<ShaderLightingResult>`：BRDF 计算 |

`LightingComputeComponentAsRenderComponent`（[lighting-system/src/lib.rs:88](../../content/lighting/gpu-system/lighting-system/src/lib.rs#L88)）把后三者接成 `RenderComponent`：`post_setup_pass` 依次绑几何、scene id、光照组件、表面组件；`post_build` 在片段阶段检测 `LightableSurfaceTag`——有标签才调 `compute_lights`，结果注册为 `HDRLightResult`（[lib.rs:116-137](../../content/lighting/gpu-system/lighting-system/src/lib.rs#L116)）。

### 光照组件的组合

`LightingComputeComponentGroup`（[group.rs:4](../../content/lighting/gpu-system/lighting-system/src/group.rs#L4)）实现 Vec 聚合语义：把多类光源的组件合成一份，着色器侧 `LightingComputeInvocationGroup`（[group.rs:43](../../content/lighting/gpu-system/lighting-system/src/group.rs#L43)）逐个 `compute_lights` 并累加 diffuse/specular。单光源遍历用 `light_iter_sum`（[lib.rs:147](../../content/lighting/gpu-system/lighting-system/src/lib.rs#L147)）对 `ShaderStaticArrayReadonlyIter` 求和；单点光源的便捷实现 `ShadowedPunctualLighting`（[lib.rs:187](../../content/lighting/gpu-system/lighting-system/src/lib.rs#L187)）演示了「亮度 > 0 才查阴影」的通用模式。

### 下游实现一览

| trait | 实现 | 位置 | 说明 |
| --- | --- | --- | --- |
| `LightSystemSceneProvider` | `LightingComputeComponentGroupProvider` | [gpu-base/src/light.rs:14](../../scene/rendering/gpu-base/src/light.rs#L14) | 五类光源的 Vec 聚合 |
| `LightSystemSceneProvider` | `SceneDirectionalLightingProvider` | [light_source/directional.rs:200](../../application/viewer-content/src/rendering/lighting/light_source/directional.rs#L200) | 方向光：uniform 数组 + Basic/Cascade/NoShadow 三态阴影 |
| `LightSystemSceneProvider` | `SceneSpotLightingProvider` | [light_source/spot.rs:140](../../application/viewer-content/src/rendering/lighting/light_source/spot.rs#L140) | 聚光：uniform 数组 + 可选 basic 阴影 |
| `LightSystemSceneProvider` | `ScenePointLightingProvider` | [light_source/point.rs:139](../../application/viewer-content/src/rendering/lighting/light_source/point.rs#L139) | 点光：uniform 数组 + 可选 cube 阴影 |
| `LightSystemSceneProvider` | `SceneAreaLightingProvider` | [extension/area-lighting/src/gles.rs:80](../../extension/area-lighting/src/gles.rs#L80) | 区域光：LTC LUT + uniform 数组 |
| `LightSystemSceneProvider` | `IBLLightingComponentProvider` | [light_source/ibl/mod.rs:48](../../application/viewer-content/src/rendering/lighting/light_source/ibl/mod.rs#L48) | IBL：预过滤立方图 + BRDF LUT + 强度 uniform |
| `LightingComputeComponent` | `DirectionalLightingShader` / `SpotLightShader` / `PointLightShader` | [directional.rs:255](../../application/viewer-content/src/rendering/lighting/light_source/directional.rs#L255)、[spot.rs:177](../../application/viewer-content/src/rendering/lighting/light_source/spot.rs#L177)、[point.rs:175](../../application/viewer-content/src/rendering/lighting/light_source/point.rs#L175) | viewer 三类点光源组件（阴影是否启用进管线哈希） |
| `LightingComputeComponent` | `LTCLightingComputeComponent` | [area-lighting/src/gles.rs:105](../../extension/area-lighting/src/gles.rs#L105) | 区域光组件（LTC） |
| `LightingComputeComponent` | `IBLLightingComponent` | [ibl/src/lighting.rs:8](../../content/lighting/ibl/src/lighting.rs#L8) | IBL 组件（立方图采样） |
| `LightableSurfaceProvider` | `LightableSurfaceShadingLogicProviderAsLightableSurfaceProvider<PhysicalShading>` | [lighting-system/src/lib.rs:33](../../content/lighting/gpu-system/lighting-system/src/lib.rs#L33) + [direct_lighting.rs:11](../../content/lighting/transport/src/surface/microfacet/device/direct_lighting.rs#L11) | PBR 表面（默认） |
| `LightableSurfaceProvider` | `...<PhongShading>` | [phong.rs:5](../../content/lighting/transport/src/surface/phong.rs#L5) | Phong 表面（`SimplePhong` 选项） |
| `LightableSurfaceProvider` | `FrameGeneralMaterialBufferReconstructSurface` | [light_pass/defer_protocol.rs:139](../../application/viewer-content/src/rendering/lighting/light_pass/defer_protocol.rs#L139) | 延迟路径：从材质缓冲解码表面 |
| `GeometryCtxProvider` | `DirectGeometryProvider` | [lighting-system/src/lib.rs:53](../../content/lighting/gpu-system/lighting-system/src/lib.rs#L53) | 前向：顶点位置/法线插值 |
| `GeometryCtxProvider` | `FrameGeometryBufferReconstructGeometryCtx` | [g_buffer.rs:221](../../application/viewer-content/src/rendering/g_buffer.rs#L221) | 延迟路径：从 g-buffer 重建几何 |
| `LightingComputeInvocation` | `DirectionalLightingInvocation` 等 | [directional.rs:300](../../application/viewer-content/src/rendering/lighting/light_source/directional.rs#L300) | 逐光源遍历 uniform 数组，`light_iter_sum` 累加 |

### 阴影侧 trait

[shadow-map/src/lib.rs](../../content/lighting/gpu-system/shadow-map/src/lib.rs) 定义阴影的两个抽象：

- `AbstractShadowMapGPUData`（[lib.rs:66](../../content/lighting/gpu-system/shadow-map/src/lib.rs#L66)）：`check_rebuild`（图集尺寸变化时重建）、`clear_shadow_map`、`update_shadow_map`（一次阴影区域绘制）、`create_abstract_shadow_computer`（产出采样器）。实现：`PCFShadowMapGPUData`（[depth_atlas.rs:4](../../content/lighting/gpu-system/shadow-map/src/depth_atlas.rs#L4)）与 `VSMShadowMap`（[vsm/depth_atlas.rs:35](../../content/lighting/gpu-system/shadow-map/src/vsm/depth_atlas.rs#L35)）。
- `AbstractShadowComputer`（[lib.rs:35](../../content/lighting/gpu-system/shadow-map/src/lib.rs#L35)）：`AbstractBindingSource` + `ShaderHashProvider` + `AbstractShaderBindingSource<ShaderBindResult = Box<dyn AbstractShadowComputerInvocation>>`。`AbstractShadowComputerInvocation::compute_shadow(shadow_position, screen_position, map_info, cascade_scale, proj_linear_depth_recover_helper)`（[lib.rs:48](../../content/lighting/gpu-system/shadow-map/src/lib.rs#L48)）是过滤的着色器侧入口。实现：`PCFComputer`（[pcf_sampling/mod.rs:12](../../content/lighting/gpu-system/shadow-map/src/pcf_sampling/mod.rs#L12)）与 `VSMComputer`（[vsm/sample.rs:49](../../content/lighting/gpu-system/shadow-map/src/vsm/sample.rs#L49)）。

`use_shadow_map`（[viewer light_source/mod.rs:28](../../application/viewer-content/src/rendering/lighting/light_source/mod.rs#L28)）按 `LightSystem.filter_ty` 二选一创建，并把 `LightSystem` 的 PCF/VSM 配置以 uniform 形式同步下去（运行时改参数不重编译着色器）。`ShadowOcclusionQuery`（[lib.rs:149](../../content/lighting/gpu-system/shadow-map/src/lib.rs#L149)）是单光源阴影查询的通用 trait，`CascadeShadowMapSingleInvocation` 实现了它（[cascade.rs:382](../../content/lighting/gpu-system/shadow-map/src/cascade.rs#L382)）。

## 通道与标签语义

### 通道（fragment 语义）

材质在片段阶段 `register` 的通道定义在 [shader/api/src/graphics/semantic.rs:295](../../shader/api/src/graphics/semantic.rs#L295) 与 [direct_lighting.rs:3](../../content/lighting/transport/src/surface/microfacet/device/direct_lighting.rs#L3)：

| 通道 | 类型 | 谁注册 | 谁消费 |
| --- | --- | --- | --- |
| `ColorChannel` | Vec3 | 材质（PbrMR 的 base_color 等） | `PhysicalShading::construct_shading_impl` 取 albedo |
| `EmissiveChannel` | Vec3 | 材质 | `ForwardLightingEmissiveAdd` 加进 HDR |
| `MetallicChannel` / `RoughnessChannel` / `ReflectanceChannel` | f32 | PbrMR / PbrSG | 构造 `ShaderPhysicalShading` |
| `SpecularChannel` | Vec3 | PbrSG（specular workflow） | 有它时走 specular 工作流（metallic = max channel） |
| `ShininessChannel` | f32 | occ 材质 | `PhongShading` |
| `AlphaChannel` | f32 | 材质 | `LDROutput` 取 alpha |
| `HDRLightResult` / `LDRLightResult` / `ShouldUsePreSetLDRResult` | Vec3 / Vec3 / bool | 光照组件 / tonemap | 光照链中间值，见下 |
| `DefaultDisplay` | Vec4 | 材质（color）或 `LDROutput`（光照结果） | `DefaultDisplayWriter` 写最终输出 |

### 标签

- `LightableSurfaceTag`（[lighting-system/src/lib.rs:86](../../content/lighting/gpu-system/lighting-system/src/lib.rs#L86)）：标记「材质可被光照」。注释特别说明：无论表面配置是否可光照，这个标签独立决定是否接入光照——Unlit 材质不注册它，光照 pass 检测不到就跳过光照（Unlit 的 unlit 标签见 [gles-material-host-render-guide.md](gles-material-host-render-guide.md) 的材质表格）。
- 材质类型标签：`PbrMRMaterialTag` / `PbrSGMaterialTag` / `UnlitMaterialTag` / `OccSurfaceTag`——延迟光照的 encode/decode 按这些标签分派（见下节）。

### 光照链的语义接线

`LightingComputeComponentAsRenderComponent::post_build`（[lib.rs:116](../../content/lighting/gpu-system/lighting-system/src/lib.rs#L116)）把光照结果注册为 `HDRLightResult`；`ForwardLightingEmissiveAdd`（[tonemap.rs:109](../../content/texture/gpu-process/src/tonemap.rs#L109)）把 `EmissiveChannel` 加进 HDR；`ToneMap::post_build`（[tonemap.rs:82](../../content/texture/gpu-process/src/tonemap.rs#L82)）查 `ShouldUsePreSetLDRResult`——延迟路径下 Unlit 表面已预设 LDR 结果则不覆盖，否则把 HDR tonemap 成 `LDRLightResult`；`LDROutput`（[mod.rs:477](../../application/viewer-content/src/rendering/lighting/mod.rs#L477)）把 `LDRLightResult` + `AlphaChannel` 合成 `DefaultDisplay`（只在 `LightableSurfaceTag` 存在时）。`DefaultDisplayWriter`（[mod.rs:497](../../application/viewer-content/src/rendering/lighting/mod.rs#L497)）在 pass 描述上追加颜色附件并把 `DefaultDisplay` 写入——这就是 `GLESScenePassContent` 里 `disable_auto_write` 之后由谁写颜色输出的答案（见 [gles-material-host-render-guide.md](gles-material-host-render-guide.md) 的「GLESSceneRenderer 与 PassContent」）。

## per-scene 光照 uniform 数组

### 数据结构

[gpu-gles/src/light/mod.rs](../../scene/rendering/gpu-gles/src/light/mod.rs) 虽然挂在 GLES crate 下，但它产出的 uniform 数组被**两个后端共用**（viewer 的 `use_lighting` 直接调用）：

- `PerSceneLightArray<T>`（[mod.rs:94](../../scene/rendering/gpu-gles/src/light/mod.rs#L94)）：`UniformArrayWithLengthInfo<T, LIGHT_LIST_LEN>`（[mod.rs:122](../../scene/rendering/gpu-gles/src/light/mod.rs#L122)，`length: Vec4<u32>` 只用到 .x + 定长 `Shader140Array`）+ `mapping: FastHashMap<light_id, u32>`（灯光 → 数组下标）。
- `PerSceneLightUniformArray<T>`（[mod.rs:14](../../scene/rendering/gpu-gles/src/light/mod.rs#L14)）：`scene_id → PerSceneLightArray` 的哈希表。
- `LightUniformInfo<T>`（[mod.rs:40](../../scene/rendering/gpu-gles/src/light/mod.rs#L40)）：场景 → `UniformBufferCachedDataView`（GPU 缓冲）+ `allocation_info`（场景 → 灯光 → 下标，**阴影代码靠它对齐下标**）+ label。以 `Arc<RwLock>` 共享（`SharedLightUniformInfo`）。
- `LIGHT_LIST_LEN = 8`（[mod.rs:117](../../scene/rendering/gpu-gles/src/light/mod.rs#L117)）：每场景最多 8 个同类型灯光，超限的灯光 `log::warn` 后丢弃。注释提示 `light_ref_scene` 迭代顺序应把最重要的光放前面。

### 提取与同步

每类灯光的流程同构（以方向光为例，[directional.rs:13](../../scene/rendering/gpu-gles/src/light/directional.rs#L13)）：

1. `use_shared_light_uniform_info` 创建共享表，`skip_if_not_waked` 内订阅「灯光实体变化 + 节点世界矩阵变化」（相机变化也唤醒——注释说明靠 uniform diff 处理）。
2. `create_directional_light_uniform`（[directional.rs:35](../../scene/rendering/gpu-gles/src/light/directional.rs#L35)）遍历 `DirectionalRefScene` 外键，逐灯从节点 world 矩阵算方向/位置（方向光 `world.forward().reverse().normalize()`），组装 `DirectionalLightUniform { illuminance, direction, follow_camera }`（std140，[directional.rs:6](../../scene/rendering/gpu-gles/src/light/directional.rs#L6)）。
3. `compute_light_list`（[mod.rs:19](../../scene/rendering/gpu-gles/src/light/mod.rs#L19)）按场景聚合，并**为没有灯光的场景也插入空数组**——注释说明这样减少着色器变体、保证空场景正确同步。
4. `sync_per_scene_uniforms`（[mod.rs:64](../../scene/rendering/gpu-gles/src/light/mod.rs#L64)）：`allocation_info` 整表替换；每个场景的 GPU 缓冲首次 `create`、已存在的 `set` + `upload_with_diff`（差分上传，只有变化的字段才真正写入）。

聚光/点光/区域光完全同构，uniform 结构体分别是 `SpotLightUniform`（[spot.rs:6](../../scene/rendering/gpu-gles/src/light/spot.rs#L6)，含 HPT 位置、half_cone_cos、half_penumbra_cos）、`PointLightUniform`（[point.rs:6](../../scene/rendering/gpu-gles/src/light/point.rs#L6)）、`LTCAreaLightUniform`（[area-lighting/src/gles.rs:8](../../extension/area-lighting/src/gles.rs#L8)，预计算世界空间四顶点 p1-p4）。着色器侧 `UniformArrayWithLengthInfoShaderPtr` 的 `into_shader_iter`（[mod.rs:139](../../scene/rendering/gpu-gles/src/light/mod.rs#L139)）把「length + 数组」展开成按 length 截断的只读迭代器——`light_iter_sum` 遍历它逐灯计算。

### 单位与衰减

着色器侧的入射光计算在 [content/lighting/punctual/src/lib.rs](../../content/lighting/punctual/src/lib.rs)：`PunctualShaderLight` trait 定义 `compute_incident_light(ctx) -> ENode<ShaderIncidentLight>`。方向光直接给 `illuminance`（lx，`follow_camera` 时用相机的无平移矩阵旋转方向）；点光/聚光把 `luminance_intensity`（cd）经 `punctual_light_intensity_to_illuminance_factor`（[punctual.rs:131](../../content/lighting/punctual/src/lib.rs#L131)，Frostbite 论文的 `E[window1]` 距离衰减 + 四次方 smooth cutoff）转成照度；聚光再乘 `smoothstep(half_cone_cos, half_penumbra_cos)` 角度衰减。`ShaderLightingGeometricCtx`（[surface/mod.rs:54](../../content/lighting/transport/src/surface/mod.rs#L54)）携带渲染空间位置/法线/视线、帧缓冲坐标、相机 HPT 位置与无平移矩阵——光源位置用 HPT 减相机位置，避免远距离精度问题。

## 阴影系统

### 图集打包：MultiLayerTexturePacker

阴影图集的打包在 [content/texture/packer/src/pack_2d_to_3d](../../content/texture/packer/src/pack_2d_to_3d)：

- `MultiLayerTexturePackerRaw<P>`（[mod.rs:11](../../content/texture/packer/src/pack_2d_to_3d/mod.rs#L11)）：N 个 2D 打包器（`EtagerePacker`）的数组；`pack` 从第 0 层起逐个尝试，第一个装下的层返回 `PackResult2dWithDepth { result, depth }`。
- `RemappedGrowablePacker<K>`（[remap_growable.rs:13](../../content/texture/packer/src/pack_2d_to_3d/remap_growable.rs#L13)）：按 key（灯光实体句柄）维护分配，`process` 处理「删除 → 变更/新增 → 增长」批量流程；`grow` 优先翻倍 2D 尺寸、宽度/高度到上限后只增层（[remap_growable.rs:72-107](../../content/texture/packer/src/pack_2d_to_3d/remap_growable.rs#L72)），到 `max_size` 后分配失败（打印提示）。viewer 中每个**光源类型**各自持有一个 packer 与一份图集纹理（方向光、聚光、点光各一份，互不共享），同类型的光源灯光共享同一份图集。

`prepare_basic_shadow_map_uniform`（[basic.rs:25](../../content/lighting/gpu-system/shadow-map/src/basic.rs#L25)）是打包与 uniform 生成的核心：先批量 `packer.process` 全部灯光的尺寸（注释：packer 可能增长，所以先批处理），再逐灯写 `BasicShadowMapInfo`（[basic.rs:210](../../content/lighting/gpu-system/shadow-map/src/basic.rs#L210)）——含 `map_info: ShadowMapAddressInfo`（层/尺寸/偏移）、`shadow_world_position`（HPT）、`shadow_center_without_translation_to_shadowmap_ndc`（光源中心到阴影 NDC 的矩阵）、bias 与线性深度恢复参数。每场景一份 `Shader140Array<BasicShadowMapInfo, MAX_SHADOW_COUNT>`（`MAX_SHADOW_COUNT = 8`，[lib.rs:79](../../content/lighting/gpu-system/shadow-map/src/lib.rs#L79)），**数组下标 = 灯光在 `allocation_info` 里的下标**——这就是光照 uniform 与阴影 uniform 对齐的方式。图集所需尺寸 = `packer.current_size()`，用于 `check_rebuild`。

### 图集纹理与绘制

`ShadowAtlas`（[depth_atlas.rs:86](../../content/lighting/gpu-system/shadow-map/src/depth_atlas.rs#L86)）是 `Depth32Float` 的 2D-array 纹理：`get_layer_view(layer)` 取单层视图（逐层清空/绘制），`get_full_view` 取全图集视图（着色器采样）。`PCFShadowMapGPUData::update_shadow_map`（[depth_atlas.rs:37](../../content/lighting/gpu-system/shadow-map/src/depth_atlas.rs#L37)）用单层视图开 `pass("shadow-map")` 深度附件，把 `ShadowMapDrawRequest`（含 `ShadowPassDesc`：pass 描述 + 区域地址）交给 `LightSystem::prepare` 的 `content` 闭包；`clear_shadow_map`（[depth_atlas.rs:146](../../content/lighting/gpu-system/shadow-map/src/depth_atlas.rs#L146)）逐层用清空深度附件（reverse-z 清 0，否则清 1）。注释标明「只清有分配的层」是待办——当前每帧全清。

### 三类阴影 preparer

| preparer | 位置 | 相机 | 图集区域 | 采样 |
| --- | --- | --- | --- | --- |
| `BasicShadowMapPreparer`（方向/聚光） | [basic.rs:171](../../content/lighting/gpu-system/shadow-map/src/basic.rs#L171) | 方向光：正交（`DirectionLightShadowBound` 或 ±20 米默认）；聚光：透视（fov = 2×half_cone） | 每灯一个矩形区域 | `BasicShadowMapComponent`（[basic.rs:220](../../content/lighting/gpu-system/shadow-map/src/basic.rs#L220)）按 `shadow_idx` 查 |
| `CubeShadowMapPreparer`（点光） | [cube.rs:188](../../content/lighting/gpu-system/shadow-map/src/cube.rs#L188) | 透视 90° fov，near 0.1，far = cutoff | 每灯一个 2×3 网格（6 面，[cube.rs:18](../../content/lighting/gpu-system/shadow-map/src/cube.rs#L18)） | `CubeShadowMapComponent`（[cube.rs:248](../../content/lighting/gpu-system/shadow-map/src/cube.rs#L248)）：按方向 `select_cube_face` 选面 |
| `CascadeShadowPreparer`（方向光级联） | [cascade.rs:151](../../content/lighting/gpu-system/shadow-map/src/cascade.rs#L151) | 4 个正交子视锥（[cascade.rs:11](../../content/lighting/gpu-system/shadow-map/src/cascade.rs#L11)） | 每级联一个矩形区域（同一 packer） | `CascadeShadowMapComponent`（[cascade.rs:331](../../content/lighting/gpu-system/shadow-map/src/cascade.rs#L331)）按深度 `compute_cascade_index` 选级联 |

点光的面选择 `select_cube_face`（[cube.rs:346](../../content/lighting/gpu-system/shadow-map/src/cube.rs#L346)）与 `build_cube_face_world_matrices` 的面方向约定（[cube.rs:23](../../content/lighting/gpu-system/shadow-map/src/cube.rs#L23)）保持 `CubeTextureFace` 顺序一致。

级联阴影的 host 侧在 [viewer shadow_cascade.rs:3](../../application/viewer-content/src/rendering/lighting/shadow_cascade.rs#L3)：`use_cascade_shadow_map` 按「每相机」维护一份 `CascadeShadowGPUCache`（keyed_scope 相机 id），`generate_cascade_shadow_info`（[cascade.rs:23](../../content/lighting/gpu-system/shadow-map/src/cascade.rs#L23)）从视口相机视锥角点计算 4 个 split（`compute_cascade_split_index`：linear 与 log 按 `split_linear_log_blend_ratio` 混合，[cascade.rs:257](../../content/lighting/gpu-system/shadow-map/src/cascade.rs#L257)），逐级联把「相机视锥子段在光源空间里的包围盒」作为正交投影范围。着色器侧 `query_shadow_occlusion_by_idx`（[cascade.rs:403](../../content/lighting/gpu-system/shadow-map/src/cascade.rs#L403)）先按视深选级联，`cascade_scale = 首级联范围/当前级联范围` 缩放 PCF 过滤半径使各级联的世界空间覆盖一致；`filter_across_cascades` 开启时在级联边界附近采样下一级联并 smoothstep 混合过渡（[cascade.rs:460-510](../../content/lighting/gpu-system/shadow-map/src/cascade.rs#L460)）。

### 阴影位置与 bias

`compute_shadow_position`（[basic.rs:304](../../content/lighting/gpu-system/shadow-map/src/basic.rs#L304)）把渲染空间位置变换进阴影相机 NDC 再映射到 UV：

- 高精度：`shadow_world_position`（HPT）减相机世界位置（HPT），再做 `shadow_center_without_translation_to_shadowmap_ndc` 变换——相机远距离下阴影采样不糊。
- normal bias 以 texel 为单位：`shadow_texel_world_size`（[bias.rs:86](../../content/lighting/gpu-system/shadow-map/src/bias.rs#L86)）按投影行向量与光源空间深度推出世界 texel 尺寸（正交投影退化为常数，透视投影随深度线性增长）；`compute_normal_offset`（[bias.rs:44](../../content/lighting/gpu-system/shadow-map/src/bias.rs#L44)）可选 nDotL 缩放（MJP Shadows 的 `GetShadowPosOffset`），注释承认该模式对方向光不正确（已知 todo）。
- 深度 bias 按 reverse-z 取符号（`apply_direct_depth_bias`，[bias.rs:33](../../content/lighting/gpu-system/shadow-map/src/bias.rs#L33)）。`ShadowBiasConfig` 的默认值是 bias 0、normal_bias 1 texel（[shadow.rs:19](../../application/viewer-content/src/rendering/lighting/shadow.rs#L19)）。

### PCF 过滤

`ShadowPCFMode` 四种模式（[pcf_sampling/mod.rs:79](../../content/lighting/gpu-system/shadow-map/src/pcf_sampling/mod.rs#L79)），全部对齐 MJP Shadows 参考实现：

| 模式 | 位置 | 方法 | 运行时参数 |
| --- | --- | --- | --- |
| `FixedSizePCF` | [fixed_size.rs:48](../../content/lighting/gpu-system/shadow-map/src/pcf_sampling/fixed_size.rs#L48) | GatherCmp 取 2×2 邻居（GPU Pro「Fast Conventional Shadow Filtering」），内核 3×3..9×9 编译期选择 | 无（内核尺寸进管线哈希） |
| `OptimizedPCF` | [optimized.rs:10](../../content/lighting/gpu-system/shadow-map/src/pcf_sampling/optimized.rs#L10) | The Witness 的 bilinear 加权分解（3×3 默认，最多 7×7） | 无 |
| `GridPCF` | [grid.rs:6](../../content/lighting/gpu-system/shadow-map/src/pcf_sampling/grid.rs#L6) | 网格采样 + 边缘覆盖权重，动态 filter size | `pcf_filter_size` uniform |
| `RandomDiscPCF` | [random_disc.rs:13](../../content/lighting/gpu-system/shadow-map/src/pcf_sampling/random_disc.rs#L13) | Poisson 盘随机旋转内核（`random_fn(screen_position)` 做稳定旋转种子） | `pcf_filter_size` + `pcf_num_disc_samples` uniform |

全部模式共享：比较采样器（`create_shadow_depth_sampler_desc`，[lib.rs:164](../../content/lighting/gpu-system/shadow-map/src/lib.rs#L164)，线性过滤 + reverse-z 感知的 compare 函数）、`map_uv_to_atlas_uv`（[pcf_sampling/mod.rs:221](../../content/lighting/gpu-system/shadow-map/src/pcf_sampling/mod.rs#L221)）从阴影区域 UV 映射到图集 UV、`fractional_sampling_error`（[mod.rs:232](../../content/lighting/gpu-system/shadow-map/src/pcf_sampling/mod.rs#L232)）补偿小数采样误差、可选 receiver plane depth bias（`compute_receiver_plane_depth_bias`，[mod.rs:256](../../content/lighting/gpu-system/shadow-map/src/pcf_sampling/mod.rs#L256)）。`ShadowPCFConfig` 的哈希只含模式/固定尺寸/平面 bias 三项——运行时参数走 uniform（[pcf_sampling/mod.rs:134](../../content/lighting/gpu-system/shadow-map/src/pcf_sampling/mod.rs#L134)）。注意 viewer 注释：图集无 padding 且比较采样器 clamp 到图集边缘而非区域边缘，区域外采样会串到邻居区域（未处理）。

### VSM 过滤

VSM 需要把深度转成矩再模糊，[vsm/depth_atlas.rs:296](../../content/lighting/gpu-system/shadow-map/src/vsm/depth_atlas.rs#L296) 的 `update_light` 每次阴影区域绘制后执行三个全屏 quad pass：

1. `pass("vsm-convert")`（`VsmConvertTask`，[depth_convert.rs:6](../../content/lighting/gpu-system/shadow-map/src/vsm/depth_convert.rs#L6)）：`load_texel` 读深度图层的 NDC 深度，`recover_linear_depth`（[depth_convert.rs:52](../../content/lighting/gpu-system/shadow-map/src/vsm/depth_convert.rs#L52)）从投影的 w 行区分正交/透视重建线性深度，写入 (depth, depth²) 矩（`Rg32Float` 图集，[depth_atlas.rs:200](../../content/lighting/gpu-system/shadow-map/src/vsm/depth_atlas.rs#L200)）。
2/3. `pass("vsm-blur-h")` / `pass("vsm-blur-v")`（`VsmBlurTask`，[pre_filter.rs:8](../../content/lighting/gpu-system/shadow-map/src/vsm/pre_filter.rs#L8)）：可分离盒式模糊，采样位置 clamp 在灯光区域内（不串邻居），中间结果写整图集大小的 `temp_view`（注释 todo：本应只分配最大灯光区域）。

`VSMConfig`（[depth_atlas.rs:5](../../content/lighting/gpu-system/shadow-map/src/vsm/depth_atlas.rs#L5)）的 `filter_size` / `vsm_bias` / `light_bleeding_reduction` 经 uniform 下发。着色器侧 `sample_shadow_map_vsm`（[vsm/sample.rs:115](../../content/lighting/gpu-system/shadow-map/src/vsm/sample.rs#L115)）：比较深度先 `recover_linear_depth` 到矩所在空间，再用 `vsm_chebyshev_upper_bound`（[sample.rs:36](../../content/lighting/gpu-system/shadow-map/src/vsm/sample.rs#L36)，MJP 的 one-tailed Chebyshev）求遮挡概率，`light_bleeding_reduction` 用 `vsm_linstep` 掐尾重缩放。

### 阴影在 viewer 中的装配细节

- 方向光三态：`ViewerDirectionalShadowPreparer::Basic / Cascade / NoShadow`（[directional.rs:123](../../application/viewer-content/src/rendering/lighting/light_source/directional.rs#L123)），egui 面板的 `use_cascade_shadowmap_for_directional_lights` 切换 Basic/Cascade；`enable_shadow` 关闭时走 NoShadow（uniform 数组照常，阴影组件不装配，管线哈希少一项）。
- 聚光/点光的阴影各自可选（`Option<ShadowMapPreparerEntry<...>>`，[spot.rs:96](../../application/viewer-content/src/rendering/lighting/light_source/spot.rs#L96)）；`ShadowMapPreparerEntry`（[light_source/mod.rs:9](../../application/viewer-content/src/rendering/lighting/light_source/mod.rs#L9)）把「preparer + 图集」打包，`update_shadow_maps` 消费后变成 `ShadowMapGPUDataEntry`（gpu_data + 图集）供采样阶段用。
- 阴影绘制用 `keyed_scope(&shadow_id)`，阴影序号跨帧递增——每个阴影映射一个 scope 缓存，光源增删只影响后续 scope（见 [viewer-content-frame-pipeline-guide.md](viewer-content-frame-pipeline-guide.md) 的「常见疑问」关于 scope 身份的讨论）。

## 五类光源 preparer 的装配细节

### 方向光

[light_source/directional.rs:15](../../application/viewer-content/src/rendering/lighting/light_source/directional.rs#L15)：`use_basic_shadow_map_uniform`（[directional.rs:59](../../application/viewer-content/src/rendering/lighting/light_source/directional.rs#L59)）读四个阴影组件 + world 矩阵，`shadow_info_access` 闭包产出 `BasicShadowMapInfoInput`（`follow_camera` 与阴影同时开启时打警告，[directional.rs:87](../../application/viewer-content/src/rendering/lighting/light_source/directional.rs#L87)）。渲染阶段 `SceneDirectionalLightingProvider::get_scene_lighting` 按场景取 uniform 数组，按阴影三态构造 `DirectionalLightingShader`；`DirectionalLightingInvocation`（[directional.rs:300](../../application/viewer-content/src/rendering/lighting/light_source/directional.rs#L300)）对数组逐灯把「incident × occlusion」喂给 `shading.compute_lighting_by_incident`。`follow_camera` 方向在着色器侧用相机无平移矩阵旋转（[punctual.rs:36](../../content/lighting/punctual/src/lib.rs#L36)）。

### 聚光 / 点光

[spot.rs:6](../../application/viewer-content/src/rendering/lighting/light_source/spot.rs#L6) / [point.rs:6](../../application/viewer-content/src/rendering/lighting/light_source/point.rs#L6)：与方向光同构，但阴影相机从灯光几何参数现推（聚光 fov = 2×half_cone，点光 fov = 90°、far = cutoff），且**没有级联选项**。`query_shadow_occlusion_by_idx` 需要 `fragment_position.xy()` 做 PCF 的随机种子（random disc）与接收平面偏置。

### 区域光（LTC）

viewer 侧 [light_source/area/mod.rs:3](../../application/viewer-content/src/rendering/lighting/light_source/area/mod.rs#L3) 用 `use_gpu_init` 从二进制资源加载两张 64×64 `Rgba16Float` LTC LUT（`ltc_1.bin` / `ltc_2.bin`，由 [content/lighting/ltc/examples/gen_lut.rs:8](../../content/lighting/ltc/examples/gen_lut.rs#L8) 离线生成：`fit(GGXxLTCxFit, ...)` 拟合 GGX 的 LTC 矩阵与放大因子）。着色器计算在 [ltc/src/shader.rs:24](../../content/lighting/ltc/src/shader.rs#L24) 的 `LTCxLightEval`：按 (roughness, 1-nDotV) 查 LUT 取 `min_v` 矩阵与 `t2`（BRDF shadowing/Fresnel），`ltc_evaluate_rect` / `ltc_evaluate_disk`（[shader.rs:109](../../content/lighting/ltc/src/shader.rs#L109)）对预计算的四边形顶点做多边形积分，diffuse 用单位矩阵再查一遍。只有 `ShaderPhysicalShading` 表面被支持（`downcast_ref` 失败返回 0，[ltc/src/lib.rs:33](../../content/lighting/ltc/src/lib.rs#L33)）。

### IBL

viewer 侧 [light_source/ibl/mod.rs:9](../../application/viewer-content/src/rendering/lighting/light_source/ibl/mod.rs#L9)：

- `use_ibl` 用 `use_gpu_init` 加载内嵌的 BRDF LUT 图（`brdf_lut.png`，Rgba8Unorm，注释 todo 双通道 16bit），`SceneHDRxEnvBackgroundInfo` 的 `intensity` / `transform` 变化增量写 `IblShaderInfo` uniform（diffuse/specular 强度共用同一值）。
- `use_prefilter_cube_maps`（[ibl/mod.rs:73](../../application/viewer-content/src/rendering/lighting/light_source/ibl/mod.rs#L73)）：订阅 `use_gpu_texture_cubes` 的立方图 GPU 化结果，环境图变化时用 `generate_pre_filter_map`（[ibl/prefiltering.rs:44](../../content/lighting/ibl/src/prefiltering.rs#L44)）在**同帧**生成预过滤结果——diffuse 128²（cos 半球采样），specular 256² 全 mip 链（GGX 重要性采样 + solid angle 选 mip，[prefiltering.rs:236](../../content/lighting/ibl/src/prefiltering.rs#L236)）。
- 着色器侧 `IBLLighting::compute_lights`（[ibl/lighting.rs:59](../../content/lighting/ibl/src/lighting.rs#L59)）：diffuse 采样 `sample_normal`（经 `transform` 旋转）、specular 按 roughness 选 mip 采样反射方向、BRDF LUT 按 (roughness, nDotV) 取 `(f0 * lut.x + lut.y) * specular`。只有 PBR 表面被支持（同 LTC 的 `downcast_ref` 模式）。

### 场景 id

`use_scene_id_provider`（[gpu-base/src/scene_id.rs:6](../../scene/rendering/gpu-base/src/scene_id.rs#L6)）把 `SceneEntity` 实体集合的增量变化映射成「实体 → 分配索引」uniform（`Vec4<u32>` 集合）。`LightingComputeComponentAsRenderComponent` 在片段阶段 `bind_by` 读 `.x()` 作为 scene_id 传给 `build_light_compute_invocation`——IBL 注释说明资源是在 host 侧按场景选好的，scene_id 目前只是传参。

## 前向 / 延迟光照组件装配

`use_render_lighting_scene_content`（[light_pass/mod.rs:20](../../application/viewer-content/src/rendering/lighting/light_pass/mod.rs#L20)）是场景内容上屏的统一入口，按 `LightingTechniqueKind` 分两个 scope。完整帧序列见 [viewer-content-frame-pipeline-guide.md](viewer-content-frame-pipeline-guide.md) 的「light_pass 的接入位置」，这里补组件装配细节。

### Forward

[light_pass/mod.rs:82](../../application/viewer-content/src/rendering/lighting/light_pass/mod.rs#L82)：

```rust
let mut pass_base = pass("scene forward");
let color_writer = DefaultDisplayWriter::extend_pass_desc(&mut pass_base, scene_result, color_ops);  // 追加颜色附件
let g_buffer_base_writer = g_buffer.extend_pass_desc(&mut pass_base, depth_ops, load_and_store());   // 深度 + 法线附件
let opaque_scene_pass_dispatcher = &RenderArray([
  &blend_disabler, &color_writer, &g_buffer_base_writer, pass_com,   // pass_com = [forward_lighting, clip_component]
]) as &dyn RenderComponent;
```

- `forward_lighting` 就是 `get_scene_forward_lighting_component` 的产物（[mod.rs:76-79](../../application/viewer-content/src/rendering/lighting/light_pass/mod.rs#L76)），永远提前取——延迟模式下透明 pass 还要用。
- 不透明批走 `use_draw_with_oc_maybe_enabled`（遮挡剔除 + frustum 见 [viewer-content-frame-pipeline-guide.md](viewer-content-frame-pipeline-guide.md) 的「剔除的装配」）；透明批交给 `ViewerTransparentRenderer`，与不透明共用同一 pass（NaiveAlphaBlend 时）或独立管线（OIT 时）。
- `blend_disabler` 只在「全部对象当不透明画」（`ViewerTransparentRenderer::Opaque`）时装配——否则透明对象的混合由透明渲染器自己控制。
- 材质侧的法线写进 g-buffer 法线附件（`FrameGeometryBufferPassEncoder`，[g_buffer.rs:117](../../application/viewer-content/src/rendering/g_buffer.rs#L117)），供 TAA/描边/SSAO 复用；entity_id 通道在 webgl 下降级（[g_buffer.rs:22](../../application/viewer-content/src/rendering/g_buffer.rs#L22)）。

### DeferLighting

[light_pass/mod.rs:143](../../application/viewer-content/src/rendering/lighting/light_pass/mod.rs#L143)：

**编码 pass（`scene defer encode`）**：g-buffer（深度/法线）+ 材质缓冲：

```rust
let m_buffer = FrameGeneralMaterialBuffer::new(ctx);           // 4 个附件：类型 id(R8Uint) + a/b/c(Rgba8UnormSrgb)
let indices = m_buffer.extend_pass_desc(&mut pass_base);       // 类型 id 清成 u8::MAX（背景哨兵）
let material_writer = FrameGeneralMaterialBufferEncoder { indices, materials: lighting_cx.deferred_mat_supports };
```

`FrameGeneralMaterialBufferEncoder::post_build`（[defer_protocol.rs:120](../../application/viewer-content/src/rendering/lighting/light_pass/defer_protocol.rs#L120)）按材质标签分派：注册表里的 encode 函数逐个尝试（`contains_type_tag` 首匹配），命中的把该材质的通道打包进三个颜色附件，并把**注册表下标**写进类型 id 附件。

**光照 compute pass（`deferred lighting compute`）**：全屏 quad（`draw_quad()`）：

- `FrameGeometryBufferReconstructGeometryCtx`（[g_buffer.rs:221](../../application/viewer-content/src/rendering/g_buffer.rs#L221)）实现 `GeometryCtxProvider`：从 g-buffer 读深度/法线，`shader_uv_space_to_render_space`（视图-投影逆矩阵 × UV × 深度）重建渲染空间位置。
- `FrameGeneralMaterialBufferReconstructSurface`（[defer_protocol.rs:139](../../application/viewer-content/src/rendering/lighting/light_pass/defer_protocol.rs#L139)）实现 `LightableSurfaceProvider`：读类型 id，`u8::MAX` 时 `discard`（背景不计算）；按注册表下标 `switch` 解码出 `MultiMaterialUberDecoder`——统一注册 `LightableSurfaceTag`、`LDRLightResult`（Direct 表面）与 `ShouldUsePreSetLDRResult`（tonemap 据此跳过已预设的 LDR），并把解码出的 `ShaderPhysicalShading` / `ShaderPhongShading` 作为 uber 表面：`compute_lighting_by_incident` 内部再按类型 id switch 到具体 BRDF（[defer_protocol.rs:225](../../application/viewer-content/src/rendering/lighting/light_pass/defer_protocol.rs#L225)）。
- 光照组件复用前向的 `get_scene_lighting_component`，只是 geometry/surface 换成上述两个（[light_pass/mod.rs:201](../../application/viewer-content/src/rendering/lighting/light_pass/mod.rs#L201)）。颜色写入由 `DefaultDisplayWriter { write_channel_index: 0 }` 负责。

**透明 forward pass（`scene forward transparent in defer mode`）**：[light_pass/mod.rs:222](../../application/viewer-content/src/rendering/lighting/light_pass/mod.rs#L222)——不透明对象已在 g-buffer 里，透明对象单独用前向光照画一遍（与 Forward 模式共用 `pass_com`）。

### 三种材质在延迟路径的编解码

`DeferLightingMaterialRegistry`（[defer_protocol.rs:74](../../application/viewer-content/src/rendering/lighting/light_pass/defer_protocol.rs#L74)）持有三份 `fn` 表（encode/decode/decode_alpha），`register_material_impl` 注册：

| 实现 | 标签 | channel_a | channel_b | channel_c |
| --- | --- | --- | --- | --- |
| `PbrSurfaceEncodeDecode` | `PbrMRMaterialTag` / `PbrSGMaterialTag` | (albedo, roughness) | (f0, emissive.x) | (emissive.yz, alpha, 1) |
| `UnlitSurfaceEncodeDecode` | `UnlitMaterialTag` | color（直接 LDR） | — | — |
| `PhongSurfaceEncodeDecode` | `OccSurfaceTag` | (diffuse, alpha) | (specular, emissive.x) | (emissive.yz, alpha, 1) |

alpha 在延迟模式只用于透明 pass 的解码（编码注释说明不透明路径 alpha 无意义、discard 已在编码时完成）。

## tonemap

`ToneMap`（[content/texture/gpu-process/src/tonemap.rs:4](../../content/texture/gpu-process/src/tonemap.rs#L4)）：`ToneMapType` 四种（Linear / Reinhard / Cineon / ACESFilmic，[tonemap.rs:32](../../content/texture/gpu-process/src/tonemap.rs#L32)），exposure 存 `Vec4<f32>` uniform（webgl 对齐），`update` 每帧上传。`LightSystem::prepare` 开头 `self.tonemap.update(frame_ctx.gpu)`（[lighting/mod.rs:77](../../application/viewer-content/src/rendering/lighting/mod.rs#L77)）。HDR 显示开启时强制 `ToneMapType::None`（egui 面板的逻辑，[mod.rs:395](../../application/viewer-content/src/rendering/lighting/mod.rs#L395)）。ACESFilmic 实现附注：比标准 ACES 提亮（scale 1/0.6，源自 three.js 讨论）。

## 用户视角：接入与使用

**加一盏灯**：按 [viewer-scene-building-zh.md](skill-translation/viewer-scene-building-zh.md) 的「Lights」一节建节点 + DataView 写入即可，渲染侧零注册——`use_db_entity_any_change` 唤醒 uniform 维护，灯光自动进入场景的 uniform 数组（超过 8 盏同类灯会被丢弃，log 警告）。IBL 需要给场景写 `SceneHDRxEnvBackgroundInfo` + `SceneHDRxEnvBackgroundCubeMap` 外键（[viewer-content-api-guide.md](viewer-content-api-guide.md) 的 API 也有对应封装）。阴影默认开启（`BasicShadowMapEnabledOf` 默认 true），分辨率 256²，可用 `BasicShadowMapResolutionOf` 按灯调整。

**换表面模型 / 光照技术**：改 `LightSystem.lighting_surface_ty_value`（Pbr / SimplePhong）或 `opaque_scene_content_lighting_technique`（Forward / DeferLighting），运行时经 egui 面板或直接改配置。

**扩展新光源类型**：实现 `LightSystemSceneProvider` + `LightingComputeComponent` + `LightingComputeInvocation` 三件套（区域光是完整范例：数据模型在 extension、uniform 提取在 gles.rs、组件在 [area-lighting/src/gles.rs:105](../../extension/area-lighting/src/gles.rs#L105)），再加入 `LightingComputeComponentGroupProvider` 的 `lights` 列表（[lighting/mod.rs:154](../../application/viewer-content/src/rendering/lighting/mod.rs#L154)）。

**扩展新材质**：材质着色器注册通道 + 标签即可被光照消费；若要在延迟路径也工作，还需向 `DeferLightingMaterialRegistry` 注册 encode/decode 实现（Pbr/Unlit/Phong 是范例，[defer_protocol.rs:257](../../application/viewer-content/src/rendering/lighting/light_pass/defer_protocol.rs#L257)）。

## 常见疑问

- **为什么灯光 uniform 在 gpu-gles crate 里，但 indirect 后端也用？** `PerSceneLightUniformArray` 只是「按场景聚合的 std140 uniform 数组」——与材质/网格不同，灯光数量小，无需存储缓冲投影，uniform 数组对两个后端都够用；把它放 gpu-gles 是历史归属，`use_lighting` 在两后端共用它（[frame_all.rs:440](../../application/viewer-content/src/rendering/frame_all.rs#L440)）。
- **阴影 uniform 数组为什么和灯光 uniform 数组同下标？** 两者都按 `allocation_info`（场景 → 灯光 → 下标）对齐：灯光数组下标 n 的灯光，其阴影信息也在阴影数组下标 n。`light_iter_sum` 遍历时把下标同时传给阴影查询（如 [directional.rs:311-329](../../application/viewer-content/src/rendering/lighting/light_source/directional.rs#L311)）。
- **为什么阴影 pass 也要走批提取器？** 阴影绘制就是「从阴影相机画不透明场景」，与主视图完全同构——复用 `extract_scene_batch` + `use_make_scene_batch_pass_content` 让 LOD/剔除/批提取在阴影相机下也生效（`LODCameraInfo` 按图集分辨率选级）。代价是每帧全量重绘所有阴影（注释 todo 未做按帧 dirty）。
- **为什么 emissive 只能加一次？** `ForwardLightingEmissiveAdd` 会把 `EmissiveChannel` 加进 HDR——若一次绘制里出现多个光照组件就会重复加（[mod.rs:463-465](../../application/viewer-content/src/rendering/lighting/mod.rs#L463) 的注释），所以 `get_scene_lighting_component` 的用法约定是每 pass 一份。
- **延迟模式为什么透明对象还要前向画？** g-buffer + 材质缓冲只编码不透明表面（alpha 混合的透明对象写 g-buffer 会污染深度/法线），透明对象保持前向光照独立 pass（[light_pass/mod.rs:222](../../application/viewer-content/src/rendering/lighting/light_pass/mod.rs#L222)）。
- **图集区域为什么会串色？** 图集打包无 padding + 比较采样器 clamp 到图集边缘（viewer 注释 [lighting/mod.rs:22-25](../../application/viewer-content/src/rendering/lighting/mod.rs#L22)）；PCF 的区域外采样会采到邻居区域的 texel。VSM 的模糊则显式 clamp 在灯光区域内（[pre_filter.rs:34](../../content/lighting/gpu-system/shadow-map/src/vsm/pre_filter.rs#L34)）。

## 延伸阅读

- viewer 帧内光照接入的位置与两阶段驱动：[viewer-content-frame-pipeline-guide.md](viewer-content-frame-pipeline-guide.md) 的「光照系统接入」与「light_pass 的接入位置」
- 材质侧通道/标签注册（光照的消费输入）：[gles-material-host-render-guide.md](gles-material-host-render-guide.md)、[material-indirect-render-guide.md](material-indirect-render-guide.md)
- 批提取与绘制（阴影 pass 复用的基建）：[batch-extractor-guide.md](batch-extractor-guide.md)、[gpu-indirect-batch-collector-guide.md](gpu-indirect-batch-collector-guide.md)
- 场景灯光实体的创建配方：[skill-translation/viewer-scene-building-zh.md](skill-translation/viewer-scene-building-zh.md) 的「Lights」
- 着色器侧语义注册与片段阶段：[skill-translation/shader-edsl-graphics-zh.md](skill-translation/shader-edsl-graphics-zh.md)
- 着色器表面模型（PBR 微表面 / Phong / 采样）：[content/lighting/transport](../../content/lighting/transport)、LTC 拟合工具 [content/lighting/ltc/examples/gen_lut.rs](../../content/lighting/ltc/examples/gen_lut.rs)
- IBL 预过滤与 BRDF LUT 生成：[content/lighting/ibl](../../content/lighting/ibl)
