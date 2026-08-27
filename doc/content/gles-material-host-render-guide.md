# GLES 材质与模型 Host 渲染指南（scene/rendering/gpu-gles）

本文梳理 [scene/rendering/gpu-gles/](../../scene/rendering/gpu-gles/src/lib.rs) 的材质与模型 host 渲染路径：`GLESModelMaterialRenderImpl` 的 trait 抽象、材质参数与纹理句柄如何以"每材质实体一个 uniform"的方式维护、`setup_tex` 如何在绘制调用现场把纹理直接绑定进渲染上下文（对照 GPU 侧的纹理池/bindless 间接采样），以及 `use_gles_scene_model_renderer` 如何逐场景模型组装渲染组件并发出传统绘制。这是 [material-indirect-render-guide.md](material-indirect-render-guide.md) 文末「与 host 路径（GLES）的对照」一节的展开；间接路径的存储缓冲投影、材质 id 注入与间接采样见该文，本文只做对照引用。

## 前置阅读

GLES 材质路径复用了间接路径同一套场景数据模型、GPU 组件模型与着色器 EDSL，建议先了解：

| 文档 | 内容 |
| --- | --- |
| [skill-translation/scene-core-structure-zh.md](skill-translation/scene-core-structure-zh.md) | 材质实体、StandardModel 引用外键、纹理槽双外键（`TextureWithSamplingForeignKeys`）、SceneWriter 创建流程 |
| [skill-translation/shader-edsl-graphics-zh.md](skill-translation/shader-edsl-graphics-zh.md) | GraphicsShaderProvider、顶点/片元阶段、语义注册（register / query） |
| [skill-translation/shader-edsl-binding-and-typed-container-zh.md](skill-translation/shader-edsl-binding-and-typed-container-zh.md) | bind_by / bind、UniformBufferDataView 等 GPU 资源容器与绑定契约 |
| [skill-translation/fundamental-gpu-component-model-zh.md](skill-translation/fundamental-gpu-component-model-zh.md) | RenderComponent / ShaderHashProvider / ShaderPassBuilder / RenderArray / BindingController，绑定槽与管线哈希规则 |
| [skill-translation/frame-pass-assemble-zh.md](skill-translation/frame-pass-assemble-zh.md) | pass() / render_ctx() / by()、FrameCtx、PassContent——host 渲染结果如何进入帧流水线 |
| [skill-translation/query-system-zh.md](skill-translation/query-system-zh.md) | 增量查询、use_changes、稀疏写、fanout（node world matrix 派生的底座） |
| [skill-translation/viewer-scene-building-zh.md](skill-translation/viewer-scene-building-zh.md) | 材质 DataView 创建模式、纹理写入 |
| [material-indirect-render-guide.md](material-indirect-render-guide.md) | 间接材质路径全文——本指南的平行对照文档（参数存储缓冲、纹理池/bindless 间接采样、材质 id 注入） |
| [batch-extractor-guide.md](batch-extractor-guide.md) | 场景批提取：host batch 与 device batch 的分叉、alpha 语义分组 |
| [attribute-mesh-indirect-render-guide.md](attribute-mesh-indirect-render-guide.md) | 几何侧间接渲染（本文网格 host 侧的对照） |
| [viewer-content-frame-pipeline-guide.md](viewer-content-frame-pipeline-guide.md) | viewer 应用层双后端帧装配（本 crate 的渲染器在帧内如何被驱动） |

## 模式概览

间接路径把材质投影成"数据库表的一行"驻留 GPU 存储缓冲，一次间接绘制覆盖大量实体；GLES 路径反其道而行——**每个绘制调用现场组装一份材质渲染组件**：

- **每材质实体一个 uniform**：`UniformBufferCollection` 维护"材质实体分配索引 → `UniformBufferDataView`（std140 结构体）"的映射，组件变化（`use_changes`）增量写进对应字段的字节偏移（[use_result_ext.rs:225](../../platform/graphics/webgpu-hook-utils/src/use_result_ext.rs#L225)），绝不整块重写。
- **纹理句柄双份存在**：`use_tex_watcher` 把纹理/采样器外键变化增量写进"纹理句柄 uniform"（着色器侧用 `u32::MAX` 哨兵判断有无纹理）；同时 `TextureSamplerIdView` 在渲染时现场读取外键，把句柄对传给 `setup_tex`，从全局纹理绑定系统查出真正的 GPU 纹理/采样器**直接绑定**（host 侧解析资源）。
- **逐绘制绑定**：`setup_tex` 按 `(texture_handle, sampler_handle)` 现场 `bind_texture2d` + `bind_sampler`；着色器侧 `bind_and_sample` 用同一句柄对走 `TraditionalPerDrawBindingSystem` 的原生 `texture.sample`。没有纹理的槽位绑定默认纹理，布局恒定，因此纹理有无不进管线哈希。
- **trait 分片 + 首匹配**：`GLESModelMaterialRenderImpl`（材质）、`GLESModelShapeRenderImpl`（形状）、`GLESNodeRenderImpl`（节点）各司其职，`GLESModelRenderImpl`（模型级）把它们按 std model 组合；`Vec<Box<dyn ...>>` 实现按顺序尝试、返回第一个命中的实现。
- **一模型一绘制**：`use_gles_scene_model_renderer` 产出 `GLESPreferredComOrderRenderer`，对宿主批（`HostRenderBatch`）里的每个场景模型组装一个 7 组件 `RenderArray`（pass / 纹理系统 / 模型 id / 形状 / 节点 / 相机 / 材质），以 `RenderMethod::TraditionalDraw` 提交。

与间接路径共享的底层资产：场景侧材质实体与外键语义、`TextureWithSamplingForeignKeys` 双外键、`TextureSamplerHandlePair` 句柄语义、通道（`ColorChannel` 等）与标签（`LightableSurfaceTag` 等）、`ShaderAlphaConfig` alpha 处理、`srgb4_to_linear4` 颜色空间转换——只换了"存储形态（storage array vs uniform collection）"与"采样路径（间接 vs 直接绑定）"。

## gpu-gles 在 device / Gles 双后端中的位置

### 后端选择

viewer 在 [frame_all.rs:165](../../application/viewer-content/src/rendering/frame_all.rs#L165) 按 `RasterizationRenderBackendType` 二选一装配 `raster_scene_renderer`：`Gles` → 本 crate 的 `GLESSceneRenderer`（host 渲染）；`Indirect` → `IndirectSceneRenderer`（间接渲染）。后端类型来自 `ViewerInitConfig::raster_backend_type`（[init_config.rs:12](../../application/viewer-content/src/init_config.rs#L12)，默认 `Indirect`，viewer 配置文件 `viewer_init_config.toml` 中 `raster_backend_type = "Gles" / "Indirect"`），`Indirect` 内部的 `using_host_driven_indirect_draw` 子模式（宿主驱动、间接绘制命令）与整条帧流水线的双后端分流见 [viewer-content-frame-pipeline-guide.md](viewer-content-frame-pipeline-guide.md) 的「双后端：WebGPU 间接路径与 GLES 路径的分流」，这里不再重复。

纹理系统选择随之联动（[frame_all.rs:123](../../application/viewer-content/src/rendering/frame_all.rs#L123)）：`get_suitable_texture_system_ty`（[texture/mod.rs:239](../../scene/rendering/gpu-base/src/texture/mod.rs#L239)）在非 indirect（且未启用 RTX）时固定选 `GlesSingleBinding`（`TraditionalPerDrawBindingSystem`），indirect 时按 GPU 能力在 `Bindless` / `TexturePool` 间选择（RTX 开启时 Gles 路径同样使用间接纹理系统，见「选择矩阵」的 RTX 并存说明）。三种系统实现同一组 `AbstractGPUTextureSystem` / `DynAbstractGPUTextureSystem` trait（[gpu-system/src/lib.rs:51](../../content/texture/gpu-system/src/lib.rs#L51)），GLES 材质代码对三者统一调用，互不感知。

### 批提取的 host/device 分叉

批提取器 `use_occ_host_scene_batch_extractor`（[occ-style-draw-control/src/gles.rs:3](../../extension/occ-style-draw-control/src/gles.rs#L3)，内部是 [batch_extraction.rs:46](../../scene/rendering/gpu-base/src/batch_extraction.rs#L46) 的 `use_default_scene_batch_extractor`）先按可见性、alpha 语义过滤并排序，再问渲染器一句：`SceneRenderer::indirect_batch_direct_creator()`（[gpu-base/src/lib.rs:113](../../scene/rendering/gpu-base/src/lib.rs#L113)）有实现就产出 `SceneModelRenderBatch::Device`（GPU 绘制列表，间接路径用），否则产出 `SceneModelRenderBatch::Host`（[batch.rs:7](../../scene/rendering/gpu-base/src/batch.rs#L7)，一个可迭代场景模型句柄的 `HostRenderBatch`）。`GLESSceneRenderer` 不提供 device creator，天然走 host 分支——这正是"host 渲染"名字的由来：模型列表在 CPU 侧维护，逐模型发命令。

## trait 抽象体系

host 渲染器把"一个场景模型怎么画"切成四层 trait，全部是对象安全的 `dyn` 接口，且都实现了"Vec 首匹配"语义：

| trait | 定义 | 职责 |
| --- | --- | --- |
| `SceneModelRenderer` | [gpu-base/src/lib.rs:126](../../scene/rendering/gpu-base/src/lib.rs#L126) | 渲染一个场景模型：`render_scene_model(sm, camera, pass, cx, tex)`，返回 `Result<(), UnableToRenderSceneModelError>`。gpu-base 底座，本 crate 的两层渲染器都实现它 |
| `GLESModelRenderImpl` | [std_model.rs:3](../../scene/rendering/gpu-gles/src/std_model.rs#L3) | 模型级组合器：`shape_renderable(sm, cx) -> Option<(RenderComponent, DrawCommand)>` + `material_renderable(sm, cx) -> Option<RenderComponent>` |
| `GLESModelMaterialRenderImpl` | [material/mod.rs:75](../../scene/rendering/gpu-gles/src/material/mod.rs#L75) | 材质侧：`make_component(std_model, cx) -> Option<Box<dyn RenderComponent>>` |
| `GLESModelShapeRenderImpl` | [shape/mod.rs:6](../../scene/rendering/gpu-gles/src/shape/mod.rs#L6) | 形状侧：`make_component(std_model) -> Option<(RenderComponent, DrawCommand)>` |
| `GLESNodeRenderImpl` | [node.rs:3](../../scene/rendering/gpu-gles/src/node.rs#L3) | 节点侧：`make_component(node, sm) -> Option<RenderComponent>` |

`Vec<Box<dyn T>>` 的 `impl T for Vec<...>`（如 [material/mod.rs:83](../../scene/rendering/gpu-gles/src/material/mod.rs#L83)、[std_model.rs:16](../../scene/rendering/gpu-gles/src/std_model.rs#L16)）按顺序尝试每个子实现、返回第一个 `Some`。各场景模型类型的外键互斥（普通 std model 走 `StandardModelRef*`，wide line 走 `SceneModelWideLineRenderPayload`……），恰好命中一个实现；全部落空时外层把多个 `None` 汇总成 `UnableToRenderSceneModelError::UnableToFindImpl`（[gpu-base/src/lib.rs:142](../../scene/rendering/gpu-base/src/lib.rs#L142)）。

### 下游实现一览

viewer 在 [frame_all.rs:192](../../application/viewer-content/src/rendering/frame_all.rs#L192) 装配的 GLES 实现列表（顺序即首匹配顺序）：

| trait | 实现 | 位置 | 说明 |
| --- | --- | --- | --- |
| `SceneModelRenderer` | `GLESSceneRenderer` | [scene.rs:3](../../scene/rendering/gpu-gles/src/scene.rs#L3) | 顶层：持有纹理系统 + 模型渲染器 + 错误记录器，把 `SceneModelRenderBatch` 变成 `PassContent` |
| `SceneModelRenderer` | `GLESPreferredComOrderRenderer` | [scene_model.rs:31](../../scene/rendering/gpu-gles/src/scene_model.rs#L31) | 逐模型组件组装（见下文「场景模型 host 渲染器」） |
| `GLESModelRenderImpl` | `SceneStdModelRenderer` | [std_model.rs:64](../../scene/rendering/gpu-gles/src/std_model.rs#L64) | 标准模型（材质 + attribute mesh + 蒙皮 + 状态）的组合器 |
| `GLESModelRenderImpl` | `WideLineModelGLESRenderer` | [wide-line/src/gles_draw.rs:67](../../extension/wide-line/src/gles_draw.rs#L67) | 宽线：展开成线段实例 + 单位 quad 实例化绘制；`material_renderable` 返回 `Box::new(())`（无材质组件） |
| `GLESModelRenderImpl` | `WidePointsModelGLESRenderer` | [wide-styled-points/src/gles_draw.rs](../../extension/wide-styled-points/src/gles_draw.rs) | 宽点：同上模式 |
| `GLESModelRenderImpl` | `Text3dGlesRenderer` | [text-3d/src/gles_draw.rs:39](../../extension/text-3d/src/gles_draw.rs#L39) | 3D 文本：slug 网格 GPU 资源 + 局部变换 uniform |
| `GLESModelMaterialRenderImpl` | `UnlitMaterialGlesRender` | [material/unlit.rs:35](../../scene/rendering/gpu-gles/src/material/unlit.rs#L35) | Unlit 材质 |
| `GLESModelMaterialRenderImpl` | `PbrMRMaterialGlesRenderer` | [material/mr.rs:54](../../scene/rendering/gpu-gles/src/material/mr.rs#L54) | PBR metallic-roughness |
| `GLESModelMaterialRenderImpl` | `PbrSGMaterialGlesRenderer` | [material/sg.rs:54](../../scene/rendering/gpu-gles/src/material/sg.rs#L54) | PBR specular-glossiness |
| `GLESModelMaterialRenderImpl` | `OccStyleMaterialGlesRenderer` | [occ-style-material/src/gles.rs:67](../../extension/occ-style-material/src/gles.rs#L67) | occ 风格材质（Unlit / Lighted / Zebra 三态，见 [occ-style-material/src/gles.rs:179](../../extension/occ-style-material/src/gles.rs#L179)） |
| `GLESModelShapeRenderImpl` | `GLESAttributesMeshRenderer` | [shape/attribute.rs:57](../../scene/rendering/gpu-gles/src/shape/attribute.rs#L57) | attribute mesh（顶点/索引 buffer 直绑） |
| `GLESNodeRenderImpl` | `GLESNodeRenderer` | [node.rs:25](../../scene/rendering/gpu-gles/src/node.rs#L25) | 节点 world matrix uniform |
| `GLESNodeRenderImpl` | `OverrideNodeGlesGPU` | [view-dependent-transform/src/gles_draw.rs:44](../../extension/view-dependent-transform/src/gles_draw.rs#L44) | 视图依赖变换（per-view 节点 uniform 覆盖，viewer 装配见 [frame_all.rs:217](../../application/viewer-content/src/rendering/frame_all.rs#L217)） |

## 材质 uniform 渲染路径

三类标准材质（Unlit / PbrMR / PbrSG）与 occ 材质结构完全同构，以 MR 为样板（[material/mr.rs:6](../../scene/rendering/gpu-gles/src/material/mr.rs#L6)）走完整个链路。

### use_pbr_mr_material_uniforms：UniformBufferCollection 与增量写

```rust
let uniforms = cx.use_uniform_buffers("pbr mr uniform");

cx.use_changes::<PbrMRMaterialBaseColorComponent>()
  .update_uniforms(&uniforms, offset_of!(Uniform, base_color), cx.gpu);
// ...emissive / normal_mapping_scale / roughness / metallic / alpha / alpha_cutoff 同理
```

- `use_uniform_buffers`（[hook.rs:123](../../platform/graphics/webgpu-hook-utils/src/hook.rs#L123)）创建 `UniformBufferCollection<K, T>`，即 `Arc<RwLock<UniformBufferCollectionRaw<K, T>>>`，内部是"key → `UniformBufferDataView<T>`"的哈希表（[use_result_ext.rs:5](../../platform/graphics/webgpu-hook-utils/src/use_result_ext.rs#L5)）。
- `update_uniforms`（[use_result_ext.rs:225](../../platform/graphics/webgpu-hook-utils/src/use_result_ext.rs#L225)）把组件变化（key = 材质实体句柄）映射为"该实体 uniform 缓冲的 `offset` 字节偏移处写入新值"：实体被删则移除缓冲，实体更新则 `entry` 创建或复用缓冲后 `write_at` 单字段。一个组件只写一个字段，与间接路径的稀疏写一一对应，只是落点是"每实体一个缓冲"而非"每类材质一个存储数组"。
- 注意 Unlit 的颜色组件声明为 sRGB，写入前 `collective_map(srgb4_to_linear4)`（[material/unlit.rs:4](../../scene/rendering/gpu-gles/src/material/unlit.rs#L4)），GPU 侧保持线性空间——与间接路径一致。

纹理句柄走 `use_tex_watcher` 的 uniform 变体（[material/mod.rs:55](../../scene/rendering/gpu-gles/src/material/mod.rs#L55)）：监听 `SceneTexture2dRefOf<T>` / `SceneSamplerRefOf<T>` 两个外键变化，各自 `map_u32_index_or_u32_max` 后写进纹理句柄 uniform 的 `texture_handle` / `sampler_handle` 字段偏移（`offset_of!(TextureSamplerHandlePair, ...)`）。没有绑纹理的外键值是 `None`，映射为 `u32::MAX`——着色器侧"无纹理"哨兵。这比间接路径的 `use_tex_watcher` 只差一个落点：`update_uniforms` vs `update_storage_array`。

### make_component：现场组装

渲染阶段（`cx.when_render`）持有只读视图后，`make_component`（[material/mr.rs:65](../../scene/rendering/gpu-gles/src/material/mr.rs#L65)）对一个 std model 现场组装 GPU 材质组件：

```rust
let idx = self.material_access.get(idx)?;              // sm → 材质实体（StandardModelRefPbrMRMaterial 外键）
let r = PhysicalMetallicRoughnessMaterialGPU {
  uniform: self.uniforms.get(&idx.alloc_index())?,     // 该材质的参数 uniform
  alpha_mode: self.alpha_mode.get_value(idx)?,
  base_color_alpha_tex_sampler: self.base_color_tex_sampler.get_pair(idx).unwrap_or(EMPTY_H),
  // ...其余三对纹理句柄
  texture_uniforms: self.tex_uniforms.get(&idx.alloc_index())?, // 纹理句柄 uniform
  binding_sys: cx,
};
```

- 参数 uniform 与纹理句柄 uniform 各自按材质实体分配索引取 `UniformBufferDataView`——与间接路径的"槽位"一一对应，只是从"数组下标"换成了"哈希表 key"。
- `TextureSamplerIdView::get_pair`（[material/mod.rs:111](../../scene/rendering/gpu-gles/src/material/mod.rs#L111)）在渲染时读全局数据库外键，返回 `(texture_alloc_index, sampler_alloc_index)` 宿主句柄对；外键缺失返回 `None` → 落到 `EMPTY_H = (u32::MAX, u32::MAX)`（[material/mod.rs:120](../../scene/rendering/gpu-gles/src/material/mod.rs#L120)）。
- 任何一步取不到（材质未加载、uniform 尚未写入）就返回 `None`，由外层汇总为渲染错误，`SceneModelErrorRecorder`（[error_model.rs:10](../../scene/rendering/gpu-base/src/error_model.rs#L10)）只对同一模型记一次日志。

### GPU 组件三件套

`PhysicalMetallicRoughnessMaterialGPU`（[material/mr.rs:116](../../scene/rendering/gpu-gles/src/material/mr.rs#L116)）实现组件模型的三个 trait：

- **`ShaderHashProvider`**：只哈希 `alpha_mode` + 类型 id（[mr.rs:131](../../scene/rendering/gpu-gles/src/material/mr.rs#L131)）。纹理有无、纹理数量不进哈希——绑定布局按材质类型恒定（见下）。
- **`ShaderPassBuilder::setup_pass`**（[mr.rs:138](../../scene/rendering/gpu-gles/src/material/mr.rs#L138)）：依次 `bind` 参数 uniform、纹理句柄 uniform，然后对四对句柄各调一次 `setup_tex`。绑定顺序 = 着色器侧 `bind_by` 顺序（组件模型绑定契约，见 [fundamental-gpu-component-model-zh.md](skill-translation/fundamental-gpu-component-model-zh.md)）。
- **`GraphicsShaderProvider::build`**（[mr.rs:149](../../scene/rendering/gpu-gles/src/material/mr.rs#L149)）：片元阶段 `bind_by` 两个 uniform，`bind_and_sample` 逐纹理采样合成 base_color / metallic / roughness / emissive，`apply_normal_mapping_conditional`（仅当 `bind_and_sample_enabled` 报告有纹理）扰动法线，`auto_reverse_normal_by_face_order` 按面序翻转法线，`ShaderAlphaConfig::apply` 处理三种 alpha 模式（[gpu-base/src/alpha.rs:5](../../scene/rendering/gpu-base/src/alpha.rs#L5)），最后注册通道（`ColorChannel` / `EmissiveChannel` / `MetallicChannel` / `RoughnessChannel` / `DefaultDisplay`）与标签（`PbrMRMaterialTag` + `LightableSurfaceTag`）。

### 三类材质骨架与差异

| | Unlit | PbrMR | PbrSG |
| --- | --- | --- | --- |
| 参数 uniform | color(Vec4) + alpha + alpha_cutoff | base_color + metallic/roughness + emissive + normal_scale + alpha(+cutoff) | albedo + specular + glossiness + emissive + normal_scale + alpha(+cutoff) |
| 纹理句柄槽数 | 1 | 4 | 4（结构体声明 5 个字段，`glossiness_texture` 从未被写入/采样，属遗留死字段） |
| 颜色空间 | 组件 sRGB，写入时转线性 | 组件线性 | 组件线性 |
| 通道 | DefaultDisplay | Color/Emissive/Metallic/Roughness + DefaultDisplay | Color/Specular/Emissive/Roughness(1-glossiness) + DefaultDisplay |
| 标签 | UnlitMaterialTag | PbrMRMaterialTag + LightableSurfaceTag | PbrSGMaterialTag + LightableSurfaceTag |
| UV fallback | 注入零 UV | 无 | 无 |

Unlit 的特殊处理：顶点阶段 `try_query::<GeometryUV>()` 为空时注册零 UV（[unlit.rs:100](../../scene/rendering/gpu-gles/src/material/unlit.rs#L100)），且不插入 `LightableSurfaceTag`——光照 pass 检测不到标签就跳过光照。SG 的 `RoughnessChannel = 1.0 - glossiness`（[sg.rs:230](../../scene/rendering/gpu-gles/src/material/sg.rs#L230)），glossiness 是感知值。occ 材质则把 `shade_type`（Unlit/Lighted/Zebra）与 `RasterizationStates` 覆盖哈希进管线（[occ-style-material/src/gles.rs:133](../../extension/occ-style-material/src/gles.rs#L133)），并支持正面/背面双色 diffuse。

## setup_tex 直接绑定路径

### TraditionalPerDrawBindingSystem

GLES 模式下的全局纹理绑定系统是 `TraditionalPerDrawBindingSystem`（[gpu-system/src/gles.rs:4](../../content/texture/gpu-system/src/gles.rs#L4)，由 [texture/mod.rs:54](../../scene/rendering/gpu-base/src/texture/mod.rs#L54) 的 `use_gles_texture_system` 装配）：

- `textures: BoxedDynQuery<Texture2DHandle, GPU2DTextureView>` 与 `samplers: BoxedDynQuery<SamplerHandle, GPUSamplerView>` 两张句柄查找表；纹理句柄是纹理实体在全局纹理系统里的分配索引。
- 纹理 GPU 化由 `use_gpu_texture_2ds`（[texture/d2_and_sampler.rs:14](../../scene/rendering/gpu-base/src/texture/d2_and_sampler.rs#L14)）维护：加载完成的图像生成带 mipmap 的 `GPU2DTextureView`，未加载/加载中的映射为 `default_tex`（白色 1x1 兜底）；采样器由 `use_sampler_gpus`（[d2_and_sampler.rs:4](../../scene/rendering/gpu-base/src/texture/d2_and_sampler.rs#L4)）按采样器实体分配索引生成。
- `bind_texture2d` / `bind_sampler`（[gles.rs:15](../../content/texture/gpu-system/src/gles.rs#L15)）：按句柄查出 `GPU2DTextureView` / `GPUSamplerView` 直接 `collector.bind`；句柄是 `u32::MAX` 或查不到（纹理加载中）时绑默认纹理。这是纯粹的**宿主侧现场解析**——与纹理池的 atlas 打包、bindless 的 binding array 完全不同。
- `sample_texture2d`（[gles.rs:75](../../content/texture/gpu-system/src/gles.rs#L75)）：`shader_texture_handle.sample(shader_sampler_handle, uv)`——硬件原生采样与 mip 链，`as_indirect_system` 返回 `None`（不支持间接采样）。

### 双句柄设计：host pair 与 device pair

`bind_and_sample` / `bind_and_sample_enabled`（[material/mod.rs:27](../../scene/rendering/gpu-gles/src/material/mod.rs#L27)）是间接路径 `indirect_sample` 的对称物：

- `host_pair`：make_component 时从 `TextureSamplerIdView` 现场读出的句柄对——`register_shader_texture2d` / `register_shader_sampler`（[gles.rs:36](../../content/texture/gpu-system/src/gles.rs#L36)）拿它查真实纹理/采样器，注册成 `BindingNode`。
- `device_pair`：从纹理句柄 uniform 展开的着色器侧句柄对——`has_texture = device_pair.texture_handle != u32::MAX` 是"有无纹理"的运行时判断（[material/mod.rs:52](../../scene/rendering/gpu-gles/src/material/mod.rs#L52)），`bind_and_sample` 据此 `select` 采样结果或默认值。
- 同一个 `sample_texture2d_with_shader_bind`（[gpu-system/src/lib.rs:86](../../content/texture/gpu-system/src/lib.rs#L86)）接口对三种纹理系统通用：传统路径忽略 device pair、用 host pair 解析资源；纹理池/bindless 忽略 host pair、用 device pair 做非均匀索引。这正是间接 guide 所述"句柄语义可复用"的实现机制——材质着色器代码在两条路径间完全一致。

于是纹理存在性对管线是"常量"：无论有无纹理，`setup_pass` 都绑固定数量的槽位（无纹理时绑默认纹理），着色器按运行时分支取用。管线哈希只含 `alpha_mode` + 类型 id，与间接路径的 PSO 哈希内容一致（材质侧不看纹理）。

## 场景模型 host 渲染器：use_gles_scene_model_renderer

### 场景模型 id 写入

`use_gles_scene_model_renderer`（[scene_model.rs:3](../../scene/rendering/gpu-gles/src/scene_model.rs#L3)）先维护一张"场景模型 → 分配索引"的 uniform 表：`use_query_set::<SceneModelEntity>()` 的实体集合变化经 `delta_key_as_value().delta_map_value(|v| v.index())` 变成"实体 → 分配索引"，`update_uniforms` 写入每实体一个 `Vec4<u32>` 的 uniform（[scene_model.rs:10](../../scene/rendering/gpu-gles/src/scene_model.rs#L10)）。`SceneModelIdWriter`（[scene_model.rs:38](../../scene/rendering/gpu-gles/src/scene_model.rs#L38)）在顶点阶段把它展开为 `LogicalRenderEntityId` 与 `RootLogicalRenderEntityId` 两个语义——与间接路径 id 池注入的是同一个"逻辑渲染实体 id"，拾取/描边/光照的 id buffer 语义一致。

### render_scene_model 的组件组装

`GLESPreferredComOrderRenderer::render_scene_model`（[scene_model.rs:79](../../scene/rendering/gpu-gles/src/scene_model.rs#L79)）对一个场景模型走完五步：

1. 取场景模型 id uniform，构造 `SceneModelIdWriter`。
2. `node` 外键 → `GLESNodeRenderImpl::make_component` 得节点组件（视图依赖变换包装器在这里可能替换为 per-view 覆盖，见 [view-dependent-transform/src/gles_draw.rs:57](../../extension/view-dependent-transform/src/gles_draw.rs#L57)）。
3. `GLESModelRenderImpl::shape_renderable` 得形状组件 + `DrawCommand`（顶点/索引缓冲在此 `set_vertex_buffer` / `set_index_buffer`）。
4. `GLESModelRenderImpl::material_renderable` 得材质组件。
5. 组装 7 组件 `RenderArray`（[scene_model.rs:118](../../scene/rendering/gpu-gles/src/scene_model.rs#L118)）：

```rust
let contents: [BindingController<&dyn RenderComponent>; 7] = [
  pass.into_assign_binding_index(0),      // 帧 pass 组件（含光照、色调映射等）
  tex.into_assign_binding_index(0),       // 纹理绑定系统自身（传统路径为空操作）
  id.into_assign_binding_index(2),        // 场景模型 id
  shape.into_assign_binding_index(2),     // 形状（不占绑定槽，只设顶点缓冲）
  node.into_assign_binding_index(2),      // 节点 world matrix uniform
  camera.into_assign_binding_index(1),    // 相机 uniform
  material.into_assign_binding_index(2),  // 材质 uniform + 纹理直绑
];
let render = Box::new(RenderArray(contents)) as Box<dyn RenderComponent>;
render.render(cx, RenderMethod::TraditionalDraw(draw));
```

`BindingController`（[webgpu/src/rendering.rs:299](../../platform/graphics/webgpu/src/rendering.rs#L299)）把组件的全部绑定圈定到一个绑定槽（bind group 下标），槽内绑定顺序 = 数组顺序 = 着色器侧 `bind_by` 顺序；管线哈希是各组件哈希之和。材质组件在槽 2 里跟在节点 uniform 之后绑定。每一步失败都以类型化错误返回，最外层由 `SceneModelErrorRecorder` 过滤重复日志（[scene.rs:19](../../scene/rendering/gpu-gles/src/scene.rs#L19)）。

### GLESSceneRenderer 与 PassContent

`GLESSceneRenderer`（[scene.rs:3](../../scene/rendering/gpu-gles/src/scene.rs#L3)）实现 `SceneRenderer`：`use_make_scene_batch_pass_content` 要求 batch 是 host 批（`get_host_batch().unwrap()`），产出一个 `SceneRendererPassContentSource`（[gpu-base/src/lib.rs:95](../../scene/rendering/gpu-base/src/lib.rs#L95)）。`GLESScenePassContent::render`（[scene.rs:67](../../scene/rendering/gpu-gles/src/scene.rs#L67)）：

```rust
let base = default_dispatcher(pass, self.renderer.reversed_depth).disable_auto_write();
let p = RenderArray([&base, self.pass] as [&dyn RenderComponent; 2]);
for sm in self.batch.iter_scene_models() {
  let _ = self.renderer.render_scene_model(sm, &self.camera, &p, &mut pass.ctx, &self.renderer.texture_system);
}
```

`default_dispatcher`（[webgpu/src/frame/pass_base.rs:3](../../platform/graphics/webgpu/src/frame/pass_base.rs#L3)）提供 pass 信息（格式、reversed-z）并默认自动写 `DefaultDisplay` 到 0 号输出；`disable_auto_write` 关掉它，因为真正的输出写入（`DefaultDisplayWriter`、g-buffer 写入器、混合控制）由帧侧注入的 pass 组件完成。该 PassContent 在视口渲染中被 `use_render_lighting_scene_content`（[viewer-content/src/rendering/lighting/light_pass/mod.rs:20](../../application/viewer-content/src/rendering/lighting/light_pass/mod.rs#L20)）以"前向光照组件 + 裁剪组件"作为 pass 参数消费——材质注册的通道与 `LightableSurfaceTag` 由共享的光照系统消费（与间接路径同一套，见 [material-indirect-render-guide.md](material-indirect-render-guide.md)）。遮挡剔除的 occluder/subject 遍也复用 `SceneRenderer` 接口（见 [occlusion-culling-guide.md](occlusion-culling-guide.md)）。

## 网格 / 节点 / 蒙皮 / 状态侧

### attribute mesh 渲染器

`use_attribute_mesh_renderer`（[shape/attribute.rs:7](../../scene/rendering/gpu-gles/src/shape/attribute.rs#L7)）把 `create_sub_buffer_changes_from_mesh_changes`（[scene/core/src/mesh.rs:283](../../scene/core/src/mesh.rs#L283)）转出的"关系句柄 → 顶点/索引数据"变化，经 `maintain_shared_map` 维护成两张 GPU buffer 表（索引、顶点各一）。`GLESAttributesMeshRenderer::make_component`（[attribute.rs:67](../../scene/rendering/gpu-gles/src/shape/attribute.rs#L67)）：

- 索引格式由 `view_byte_size / count` 反推：4 → `Uint32`、2 → `Uint16`，否则 `unreachable!`（隐式断言索引数据格式合法）。
- 预检所有顶点缓冲可访问后构造 `AttributesMeshGPU`：`draw_command`（[attribute.rs:216](../../scene/rendering/gpu-gles/src/shape/attribute.rs#L216)）按有无索引产出 `DrawCommand::Indexed` / `DrawCommand::Array`。
- `setup_pass` 逐个 `set_vertex_buffer_by_buffer_resource_view_next` 绑顶点缓冲、`set_index_buffer` 绑索引缓冲；`build` 按 `AttributeSemantic` 注册顶点布局（Position/Normal/Tangent/Color/UV 0-2/Joints 0-3/Weights 0-3），`AttributeSemantic::Foreign` 交给 viewer 传入的回调（viewer 目前传 no-op，[frame_all.rs:145](../../application/viewer-content/src/rendering/frame_all.rs#L145)）。
- 管线哈希 = 全部顶点语义 + 拓扑（[attribute.rs:156](../../scene/rendering/gpu-gles/src/shape/attribute.rs#L156)）——语义集合不同的网格分属不同 PSO。

注意 GLES 路径没有 LOD：网格数据全量上传、按需整块重建 GPU 缓冲；LOD 只在 indirect 路径生效（见 [attribute-mesh-lod-guide.md](attribute-mesh-lod-guide.md)）。

### node uniform：高精度世界矩阵

`use_node_uniforms`（[node.rs:11](../../scene/rendering/gpu-gles/src/node.rs#L11)）消费 `use_global_node_world_mat`（[scene/core/src/node.rs:83](../../scene/core/src/node.rs#L83)，基于双查询的增量世界矩阵派生）的输出，`NodeUniform::from_world_mat` 拆成三件套（[node.rs:69](../../scene/rendering/gpu-gles/src/node.rs#L69)）：`world_matrix_none_translation`（Mat4 平移分量去掉）、`world_position_hp`（高精度平移，HPT 双精度拆分）、`normal_matrix`（法线矩阵）。顶点阶段注册 `WorldNoneTranslationMatrix` / `WorldPositionHP` / `WorldNormalMatrix` 语义，并派生 `VertexRenderNormal`。`inject_uniforms` 辅助函数（[node.rs:47](../../scene/rendering/gpu-gles/src/node.rs#L47)）供纹理/扩展组件复用。

### 蒙皮：bone 矩阵纹理

`use_skin`（[skin.rs:55](../../scene/rendering/gpu-gles/src/skin.rs#L55)）把 `use_indexed_joints_offset_mats`（[scene/core/src/skin.rs:32](../../scene/core/src/skin.rs#L32)，joint world × 逆绑定矩阵）的增量变化汇总进 `SkinBoneMatrixesGPU`（[skin.rs:81](../../scene/rendering/gpu-gles/src/skin.rs#L81)）。注释明确说明：**GLES 模式下骨矩阵必须用纹理存储**（`Rgba32Float` 2D 纹理，宽度 = 关节数 × 4，一个 Mat4 占 4 个 texel，[skin.rs:130](../../scene/rendering/gpu-gles/src/skin.rs#L130)）——顶点阶段按关节索引 `load_texel` 取矩阵（`BoneMatrixInvocationProvider`，[skin.rs:14](../../scene/rendering/gpu-gles/src/skin.rs#L14)）。取矩阵的抽象是 gpu-base 的 `BoneMatrixAccessInvocation` trait，`BoneMatrixProvider` 在着色器构建时把实现注册进语义注册表的 `any_map`（[skin.rs:42](../../scene/rendering/gpu-gles/src/skin.rs#L42)）；`SceneStdModelRenderer::shape_renderable`（[std_model.rs:73](../../scene/rendering/gpu-gles/src/std_model.rs#L73)）在存在蒙皮时把 `[bones, base_shape, state, SkinVertexTransform]` 包成 `RenderArray`，共享的 `SkinVertexTransform`（[gpu-base/src/skin.rs:7](../../scene/rendering/gpu-base/src/skin.rs#L7)）按关节索引/权重混合骨矩阵并变换位置与法线。目前骨矩阵 provider 只在 GLES 路径接通，indirect 路径仅预留了 skin id 槽位。

### 状态覆盖

`use_state_overrides`（[gpu-base/src/state.rs:23](../../scene/rendering/gpu-base/src/state.rs#L23)）把 `StandardModelRasterizationOverride` 的 `RasterizationStates` 经 `ValueInterning` 去重为 `InternedId`，`StateGPUImpl`（[state.rs:55](../../scene/rendering/gpu-base/src/state.rs#L55)）把 intern id 哈希进管线，并在 `build` 里按覆盖值改写面序/剔除（顶点侧）与混合/深度/模板（片元侧，`apply_pipeline_frag_builder`）。它与 indirect 路径的 occ-style-draw-control 分层绘制（layer/priority 排序）互补：排序发生在批提取，状态覆盖发生在 PSO。

## host 与 indirect/device 路径的分工与选择时机

### 选择矩阵

| 决策 | GLES host 路径 | Indirect 路径（device / host-driven） |
| --- | --- | --- |
| 批次来源 | `SceneModelRenderBatch::Host`（CPU 侧句柄列表） | `SceneModelRenderBatch::Device`（GPU 绘制列表）或 host 批 + `classify_draws` 现场建 id 池 |
| 绘制命令 | 每模型一个 `TraditionalDraw` | 一次 `MultiDrawIndirectCount` 覆盖批内全部模型 |
| 材质存储 | 每材质实体一个 std140 uniform（`UniformBufferCollection`） | 每类材质一个存储数组（参数 + 句柄双缓冲） |
| 纹理采样 | 句柄对现场解析 + 硬件原生采样 | 句柄入着色器，纹理池手动采样或 bindless 非均匀索引 |
| PSO 哈希（材质侧） | `alpha_mode` + 类型 id | `alpha_mode` + 类型 id（一致） |
| 顶点阶段注入 | `SceneModelIdWriter`（实体分配索引） | `SceneStdModelIdInjector`（绘制 id → std model 元数据表） |
| 蒙皮 | bone 矩阵纹理（唯一实现） | 元数据表有 skin id 槽位（`SceneStdModelStorage.skin` / `IndirectSkinId`），但尚未接入骨矩阵 provider |
| LOD | 无 | 有（attribute-mesh-lod） |
| 遮挡剔除 | 无 GPU 端剔除（可用 host 批过滤） | GPU 两遍剔除（见 occlusion-culling-guide） |
| 适用场景 | 模型数量少、实现简单、便于调试与扩展（新模型类型 = 新 trait 实现） | 模型数量大、需 GPU 剔除/LOD/批量提交 |

选择时机在 `ViewerInitConfig::raster_backend_type`（配置项 `raster_backend_type`）与 `using_host_driven_indirect_draw`（宿主驱动间接绘制：indirect 后端 + host 批，见 [gpu-indirect-batch-collector-guide.md](gpu-indirect-batch-collector-guide.md)）。`prefer_bindless_for_indirect_texture_system` 只影响 indirect 模式的纹理系统类型。GLES 与 RTX 可并存：GLES 光栅 + RTX 时，[frame_all.rs:445](../../application/viewer-content/src/rendering/frame_all.rs#L445) 会额外创建 indirect 材质存储与网格供 RTX 使用——同材质实体在两条路径各有一份 GPU 投影。

### 什么时候该用哪条路径

- 需要 GPU 端遮挡剔除、LOD、超大场景 → `Indirect`。
- 调试材质/着色器、实现简单扩展（如新模型类型只实现一个 `GLESModelRenderImpl`）、或目标设备间接能力弱（无 MIDC、无 bindless）→ `Gles`。
- 数据量中等但想要间接绘制的批量优势、又不想引入 GPU 剔除依赖 → `Indirect` + `using_host_driven_indirect_draw = true`。

## 用户视角：接入与使用

材质与网格的创建完全复用场景侧 API（与 indirect 相同，见 [material-indirect-render-guide.md](material-indirect-render-guide.md) 的「用户视角」）：`PhysicalMetallicRoughnessMaterialDataView` / `Texture2DWithSamplingDataView` 写入数据库后，渲染侧无需任何注册——组件变化驱动 uniform 增量写，纹理外键驱动句柄写，`alpha_mode` 变化驱动换管线。viewer 中把配置切到 GLES 即可看到 host 路径生效：

```toml
raster_backend_type = "Gles"
```

GLES 材质渲染器的装配（若要在 viewer 之外复用）：按 [frame_all.rs:165](../../application/viewer-content/src/rendering/frame_all.rs#L165) 的顺序调用 `use_unlit_material_uniforms` / `use_pbr_mr_material_uniforms` / `use_pbr_sg_material_uniforms` /（可选）`use_occ_material_uniforms`，收集成 `Vec<Box<dyn GLESModelMaterialRenderImpl>>`；`use_attribute_mesh_renderer` 得形状实现；`std_model_renderer` 组合成 `GLESModelRenderImpl`；再与宽线/宽点/文本实现收集成模型级 Vec；`use_node_uniforms`（可包视图依赖变换）得节点实现；最后 `use_gles_scene_model_renderer` + `GLESSceneRenderer` 产出 `SceneRenderer`。光照/帧流水线侧无需改动——通道与标签是两后端共享的接口。

## 延伸阅读

- 间接材质路径全文（存储缓冲投影、纹理池/bindless、材质 id 注入）：[material-indirect-render-guide.md](material-indirect-render-guide.md)
- 几何侧间接渲染（顶点池、两跳寻址）：[attribute-mesh-indirect-render-guide.md](attribute-mesh-indirect-render-guide.md)
- 批提取与 group key（host/device 批分叉）：[batch-extractor-guide.md](batch-extractor-guide.md)
- 间接绘制链路组织与 viewer 装配：[gpu-indirect-batch-collector-guide.md](gpu-indirect-batch-collector-guide.md)
- 双后端帧流水线装配（viewer 应用层视角）：[viewer-content-frame-pipeline-guide.md](viewer-content-frame-pipeline-guide.md)，装配代码在 [application/viewer-content/src/rendering/frame_all.rs](../../application/viewer-content/src/rendering/frame_all.rs)
- 纹理系统三实现（传统直绑 / 纹理池 / bindless）：[content/texture/gpu-system/src/gles.rs](../../content/texture/gpu-system/src/gles.rs)、[pool.rs](../../content/texture/gpu-system/src/pool.rs)、[bindless.rs](../../content/texture/gpu-system/src/bindless.rs)
- 逐实体 uniform 维护：[platform/graphics/webgpu-hook-utils/src/use_result_ext.rs:5](../../platform/graphics/webgpu-hook-utils/src/use_result_ext.rs#L5)
- GLES 光照 uniform 数组基建（`PerSceneLightUniformArray`，供 area-lighting 扩展消费）：[gpu-gles/src/light/mod.rs:14](../../scene/rendering/gpu-gles/src/light/mod.rs#L14)、[extension/area-lighting/src/gles.rs:23](../../extension/area-lighting/src/gles.rs#L23)
