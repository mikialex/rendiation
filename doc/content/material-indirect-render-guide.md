# Rendiation 材质间接渲染指南（scene/rendering/gpu-indirect/src/material + std_model）

本文梳理 [scene/rendering/gpu-indirect/src/material/](../../scene/rendering/gpu-indirect/src/material/mod.rs) 的材质侧间接渲染实现：三类标准材质（Unlit / PBR metallic-roughness / PBR specular-glossiness）如何以"每个材质实体一个 GPU 存储槽位"的方式投影成参数缓冲与纹理句柄缓冲，材质 id 如何经 std model 元数据表在顶点阶段注入，纹理如何通过全局 `GPUTextureBindingSystem`（纹理池或 bindless）间接采样，以及材质 PSO 哈希如何参与管线选择。网格/几何侧（顶点池、两跳寻址、间接命令生成）见 [attribute-mesh-indirect-render-guide.md](attribute-mesh-indirect-render-guide.md)，此处只做必要引用。

## 前置阅读

间接材质的实现建立在数据库组件变化、GPU 组件模型、着色器 EDSL 绑定与批提取之上，建议先了解：

| 文档 | 内容 |
| --- | --- |
| [skill-translation/scene-core-structure-zh.md](skill-translation/scene-core-structure-zh.md) | 材质实体（Unlit / PbrSG / PbrMR）、StandardModel 引用外键、SceneWriter 创建流程 |
| [skill-translation/shader-edsl-core-zh.md](skill-translation/shader-edsl-core-zh.md) | Node 表达式、shader 结构体、std430 布局、控制流（select_branched） |
| [skill-translation/shader-edsl-graphics-zh.md](skill-translation/shader-edsl-graphics-zh.md) | GraphicsShaderProvider、顶点/片元阶段、语义注册（register / query） |
| [skill-translation/shader-edsl-binding-and-typed-container-zh.md](skill-translation/shader-edsl-binding-and-typed-container-zh.md) | bind_by / bind、StorageBufferReadonlyDataView 等 GPU 资源容器 |
| [skill-translation/fundamental-gpu-component-model-zh.md](skill-translation/fundamental-gpu-component-model-zh.md) | ShaderHashProvider / ShaderPassBuilder / RenderComponent 与管线哈希规则 |
| [skill-translation/query-system-zh.md](skill-translation/query-system-zh.md) | 组件变化查询（use_changes）、稀疏写、fanout |
| [skill-translation/viewer-scene-building-zh.md](skill-translation/viewer-scene-building-zh.md) | 材质 DataView 创建模式、纹理写入（TexSamplerWriter） |
| [batch-extractor-guide.md](batch-extractor-guide.md) | 增量 PSO key 与 id 池分桶——材质的 alpha 语义如何决定子列表分组 |
| [attribute-mesh-indirect-render-guide.md](attribute-mesh-indirect-render-guide.md) | 几何侧：顶点池、两跳寻址、间接命令生成（本文的平行文档） |
| [indirect-draw-command-guide.md](indirect-draw-command-guide.md) | 间接绘制命令抽象与 MIDC 降级（绘制命令侧，材质不参与降级决策） |

## 模式概览

普通渲染（GLES 路径）中每个绘制调用现场绑定一份材质 uniform；间接渲染不行——一次间接绘制覆盖大量场景模型，材质参数不可能逐实例绑定。本 crate 的做法是**把材质当"数据库表的一行"投影成 GPU 常驻存储缓冲的一个槽位**，槽位下标就是材质实体的分配索引，着色器按 id 取数：

- **两类存储缓冲**：每类材质各有一个"参数缓冲"（颜色、标量、alpha）与一个"纹理句柄缓冲"。纹理句柄是 `(texture_handle, sampler_handle)` 两个 u32，指向全局纹理绑定系统里的分配索引，而不是绑定对象本身。
- **增量稀疏写**：材质组件的每次数据库变更（`use_changes`）被映射为"实体分配索引 → 新值"的稀疏写，落进对应字段的字节偏移，绝不整块重写（机制见 [use_result_ext.rs:120](../../platform/graphics/webgpu-hook-utils/src/use_result_ext.rs#L120)）。
- **统一 trait 与首匹配**：三类材质渲染器实现同一个 `IndirectModelMaterialRenderImpl`，容器按顺序遍历、返回第一个匹配的材质（三类材质外键互斥，恰好一个命中）。
- **纹理经全局绑定系统间接采样**：`indirect_sample` 先查句柄是否为 `u32::MAX` 哨兵（无纹理），有纹理时经 `AbstractIndirectGPUTextureSystem` 采样——纹理池（atlas 手动采样）或 bindless（binding array）由运行时能力决定。
- **PSO 哈希只看 alpha_mode**：有无纹理、纹理数量都不进入管线哈希（统一走运行时分支），唯一影响材质的管线变化是 `AlphaMode`；它同时进入 `hash_pipeline` 与批提取的 group key，保证分桶与管线一致。
- **std model 元数据表**：`SceneStdModelStorage { mesh, material, skin }` 把 sm → std model 的映射与材质 id 一并放在 GPU，顶点阶段从绘制 id 反查并注入 `IndirectAbstractMaterialId`。

## 核心概念

| 概念 | 定义位置 | 说明 |
| --- | --- | --- |
| `IndirectModelMaterialRenderImpl` | [material/mod.rs:98](../../scene/rendering/gpu-indirect/src/material/mod.rs#L98) | 材质渲染器统一入口：按 std model 产出材质 `RenderComponent`、哈希材质侧 group key |
| `IndirectAbstractMaterialId` | [material/mod.rs:12](../../scene/rendering/gpu-indirect/src/material/mod.rs#L12) | 双阶段语义：材质实体分配索引，片元阶段按它索引存储缓冲 |
| `TextureSamplerHandlePair` | [material/mod.rs:14](../../scene/rendering/gpu-indirect/src/material/mod.rs#L14) | 纹理句柄对：`(texture_handle, sampler_handle)` 两个 u32 |
| `indirect_sample` / `indirect_sample_enabled` | [material/mod.rs:22](../../scene/rendering/gpu-indirect/src/material/mod.rs#L22) | 间接采样封装：哨兵判断 + base level 计算 + 分支采样 |
| `use_tex_watcher` | [material/mod.rs:58](../../scene/rendering/gpu-indirect/src/material/mod.rs#L58) | 纹理/采样器外键变化 → 句柄稀疏写（`u32::MAX` 表示空槽） |
| `UnlitMaterialIndirectRenderer` | [material/unlit.rs:37](../../scene/rendering/gpu-indirect/src/material/unlit.rs#L37) | Unlit 材质渲染器：颜色 + 单纹理 |
| `PbrMRMaterialIndirectRenderer` | [material/mr.rs:58](../../scene/rendering/gpu-indirect/src/material/mr.rs#L58) | metallic-roughness 渲染器 |
| `PbrSGMaterialIndirectRenderer` | [material/sg.rs:58](../../scene/rendering/gpu-indirect/src/material/sg.rs#L58) | specular-glossiness 渲染器 |
| `PhysicalMetallicRoughnessMaterialStorage` | [material/mr.rs:100](../../scene/rendering/gpu-indirect/src/material/mr.rs#L100) | MR 参数槽：base_color / emissive / roughness / metallic / normal_scale / alpha_cutoff / alpha |
| `PhysicalMetallicRoughnessMaterialTextureHandlesStorage` | [material/mr.rs:115](../../scene/rendering/gpu-indirect/src/material/mr.rs#L115) | MR 纹理句柄槽：base_color_alpha / emissive / metallic_roughness / normal 四对句柄 |
| `SceneStdModelIndirectRenderer` | [std_model.rs:342](../../scene/rendering/gpu-indirect/src/std_model.rs#L342) | std model 渲染器：材质 + 形状 + 状态三路组合 |
| `SceneStdModelStorage` | [std_model.rs:487](../../scene/rendering/gpu-indirect/src/std_model.rs#L487) | 每 std model 一槽：`{ mesh, material, skin }` 三个 u32 id |
| `SceneStdModelIdInjector` | [std_model.rs:410](../../scene/rendering/gpu-indirect/src/std_model.rs#L410) | 顶点阶段注入组件：绘制 id → std model id → 材质/网格/蒙皮 id |
| `GPUTextureBindingSystem` | [gpu-system/src/lib.rs:160](../../content/texture/gpu-system/src/lib.rs#L160) | 全局纹理绑定系统的对象安全 trait（内部是 `DynAbstractGPUTextureSystem`） |
| `AbstractIndirectGPUTextureSystem` | [gpu-system/src/lib.rs:22](../../content/texture/gpu-system/src/lib.rs#L22) | 间接采样抽象：`compute_base_level` + `sample_texture2d_indirect` |
| `TexturePool` | [gpu-system/src/pool.rs:367](../../content/texture/gpu-system/src/pool.rs#L367) | 纹理池实现：2D array atlas + 地址元数据表 + 手动采样 |
| `BindlessTextureSystem` | [gpu-system/src/bindless.rs:30](../../content/texture/gpu-system/src/bindless.rs#L30) | bindless 实现：texture/sampler binding array + 非均匀索引 |
| `ShaderAlphaConfig` | [gpu-base/src/alpha.rs:7](../../scene/rendering/gpu-base/src/alpha.rs#L7) | 三种 alpha 模式的片元处理（discard / 混合状态 / 不变） |
| `AlphaMode` | [scene/core/src/material.rs:449](../../scene/core/src/material.rs#L449) | Opaque / Mask / Blend 三态，进入 PSO 哈希与分组 key |
| `TextureWithSamplingForeignKeys` | [scene/core/src/texture.rs:67](../../scene/core/src/texture.rs#L67) | 材质纹理槽的"纹理 + 采样器"双外键语义 |

## 分层动机与数据流

先看完整数据流，再逐层展开：

```text
用户创建材质实体（DataView → TableWriter）
  │  Texture2DWithSamplingDataView（纹理实体 + 采样器实体，经 TexSamplerWriter）
  ▼
scene/core 数据库表（组件 + 纹理/采样器外键）
  │
  ├─ use_changes::<材质组件>()           ──► 稀疏写：参数存储缓冲[实体分配索引].字段
  ├─ use_tex_watcher（FK 变化 → u32 句柄）──► 稀疏写：句柄存储缓冲[实体分配索引].字段
  │     句柄 = SceneTexture2dEntity / SceneSamplerEntity 的分配索引（或 u32::MAX）
  ▼
材质 id 表：material_key（四路 FK select）──► SceneStdModelStorage.material（按 std model 索引）
  ▼
绘制时（IndirectScenePassContent → render_indirect_batch_models）
  ├─ draw provider（间接命令，见 indirect-draw-command-guide）
  ├─ model_info_injector：顶点阶段 LogicalRenderEntityId → std model 槽 → IndirectAbstractMaterialId
  ├─ 材质 RenderComponent：片元阶段按 id 读参数 + 句柄
  │      indirect_sample → GPUTextureBindingSystem（纹理池 / bindless）→ 颜色通道
  └─ 光照 pass：contains_type_tag::<LightableSurfaceTag> → 通道参与光照计算
```

分层动机：

- **实体行与绘制解耦**。材质参数按"材质实体"而非"绘制调用"存放，一次间接绘制覆盖的任意实体都能用同一个参数缓冲，只是 id 不同；新增材质实体只需表容量增长时扩容缓冲。
- **句柄与数据解耦**。纹理句柄只是 u32 分配索引，渲染实现不关心纹理是打包进 atlas 还是走 bindless——采样路径由 `GPUTextureBindingSystem` 统一抽象，GLES 路径可复用同一套句柄语义（见文末对照）。
- **变体与数据解耦**。纹理是否存在是运行时数据（`u32::MAX` 哨兵 + 分支），不是编译期变体；只有 `AlphaMode` 影响混合/裁剪逻辑，进入 PSO 哈希。这与批提取的 group key（`require_alpha_blend`）严格对应，透明与不透明实体永远分桶。
- **宿主与设备共用一套布局**。id 表、参数槽、句柄槽都是"分配索引 → 数据"，host-driven（GLES）路径与设备剔除路径共享同一份数据布局，只是写入通道不同。

## 材质实体到 GPU 存储：增量稀疏写

以 MR 为例（[material/mr.rs:6](../../scene/rendering/gpu-indirect/src/material/mr.rs#L6) 的 `use_pbr_mr_material_storage`）：

```rust
let (cx, storages) = cx.use_storage_buffer("pbr mr materials parameter data", 128, u32::MAX);
cx.use_changes::<PbrMRMaterialBaseColorComponent>()
  .update_storage_array(cx, storages, offset_of!(Storage, base_color));
// ...roughness / metallic / emissive / normal_scale / alpha / alpha_cutoff 同理
storages.use_max_item_count_by_db_entity::<PbrMRMaterialEntity>(cx);
storages.use_update(cx);
```

- `use_storage_buffer` 创建可增长的只读存储缓冲（`SparseUpdateStorageBuffer`，见 [sparse_update_storage_buffer.rs:11](../../platform/graphics/webgpu-hook-utils/src/sparse_update_storage_buffer.rs#L11)）；初始容量 128，`use_max_item_count_by_db_entity` 按材质实体表的容量增长。
- 每个 `use_changes` 把"组件变化"转换成稀疏写：key 是材质实体句柄（写入位置 = 实体分配索引），value 经 `map_u32_index_or_u32_max`（[database/hook/mod.rs:416](../../utility/database/src/hook/mod.rs#L416)）映射后写入 `field_offset` 字节偏移。所有稀疏写收集进 `SparseBufferWritesSource`，在 render 阶段统一写入——组件改一个值只写一个槽的一个字段。
- 注意 Unlit 的颜色组件声明为 sRGB，写入前映射 `srgb4_to_linear4`（[material/unlit.rs:8](../../scene/rendering/gpu-indirect/src/material/unlit.rs#L8)），保证 GPU 侧全线性空间。

纹理句柄走 `use_tex_watcher`（[material/mod.rs:58](../../scene/rendering/gpu-indirect/src/material/mod.rs#L58)）：监听 `SceneTexture2dRefOf<T>` 与 `SceneSamplerRefOf<T>` 两个外键变化（`T` 是材质的纹理槽语义，如 `PbrMRMaterialBaseColorAlphaTex`），各自映射为实体分配索引后写进 `TextureSamplerHandlePair` 的对应字段。没有绑纹理的外键值是 `None`，映射为 `u32::MAX`——这正是着色器侧判断"无纹理"的哨兵。

## 三类标准材质

### 共性骨架

三类渲染器结构完全同构（以 [material/mr.rs](../../scene/rendering/gpu-indirect/src/material/mr.rs) 为代表）：

- **存储双缓冲**：参数缓冲 + 纹理句柄缓冲，槽位布局各是一个 `#[std430_layout]` shader 结构体（MR 见 [mr.rs:100](../../scene/rendering/gpu-indirect/src/material/mr.rs#L100) 与 [mr.rs:115](../../scene/rendering/gpu-indirect/src/material/mr.rs#L115)，SG 见 [sg.rs:100](../../scene/rendering/gpu-indirect/src/material/sg.rs#L100)，Unlit 见 [unlit.rs:78](../../scene/rendering/gpu-indirect/src/material/unlit.rs#L78)）。
- **`IndirectModelMaterialRenderImpl` 实现**：`make_component_indirect` 读 `StandardModelRef{Unlit,PbrMR,PbrSG}Material` 外键拿到材质实体，构造带生命周期的 GPU 组件；`hash_shader_group_key` 把 `alpha_mode` 哈希进材质侧 group key（[mr.rs:83](../../scene/rendering/gpu-indirect/src/material/mr.rs#L83)）。
- **组件三件套**：`ShaderHashProvider` 哈希 `alpha_mode` + `TypeId`；`ShaderPassBuilder` 绑定两个存储缓冲（绑定顺序 = shader 侧 `bind_by` 顺序）；`GraphicsShaderProvider::build` 片元阶段按 `IndirectAbstractMaterialId` 取参数与句柄，做纹理合成、法线贴图、alpha 处理，最后注册通道与类型标签。
- **alpha 统一处理**：`ShaderAlphaConfig { alpha_mode, alpha_cutoff, alpha }.apply(builder)`（[gpu-base/src/alpha.rs:5](../../scene/rendering/gpu-base/src/alpha.rs#L5)）——Opaque 无事发生，Mask 按 `alpha < cutoff` discard，Blend 注册 `AlphaChannel` 并把可混合输出目标的混合状态设为 ALPHA_BLENDING。

### Unlit

[material/unlit.rs:107](../../scene/rendering/gpu-indirect/src/material/unlit.rs#L107)：

- 顶点阶段有一个特殊处理：如果几何没有提供 UV（`try_query::<GeometryUV>()` 为空），注入零 UV——unlit 材质可以用于无 UV 几何。
- 片元阶段取 `color * color_alpha_tex` 注册为 `DefaultDisplay`，插入 `UnlitMaterialTag`。**不**插入 `LightableSurfaceTag`，光照 pass 检测不到标签就跳过光照计算，输出直接是 `DefaultDisplay` 颜色。

### PBR Metallic-Roughness

[material/mr.rs:146](../../scene/rendering/gpu-indirect/src/material/mr.rs#L146)：

- 参数：`base_color`、`metallic`、`roughness`、`emissive`、`normal_mapping_scale`、`alpha_cutoff`、`alpha`；纹理：base_color_alpha、metallic_roughness、emissive、normal 四槽。
- 采样合成：base_color 乘 base_color_alpha 纹理的 rgb、alpha 乘其 w；metallic 乘 metallic_roughness 的 z 通道、roughness 乘 y 通道；emissive 乘 emissive 纹理。
- 法线贴图走 `apply_normal_mapping_conditional_uniform_cfg`（[mr.rs:196](../../scene/rendering/gpu-indirect/src/material/mr.rs#L196)），只在 `indirect_sample_enabled` 返回"有纹理"时才采样扰动法线；`auto_reverse_normal_by_face_order` 按面序翻转法线。
- 注册通道：`ColorChannel` / `EmissiveChannel` / `MetallicChannel` / `RoughnessChannel`，并注册 `DefaultDisplay`（供无光照展示）、插入 `PbrMRMaterialTag` 与 `LightableSurfaceTag`。光照 pass 通过 `contains_type_tag::<LightableSurfaceTag>` 决定是否计算光照（[lighting-system/src/lib.rs:65](../../content/lighting/gpu-system/lighting-system/src/lib.rs#L65)），表面构造器按通道查询材质参数。

### PBR Specular-Glossiness

[material/sg.rs:146](../../scene/rendering/gpu-indirect/src/material/sg.rs#L146)：

- 参数：`albedo`、`specular`、`glossiness`、`emissive`、`normal_mapping_scale`、alpha；纹理：albedo、specular_glossiness、emissive、normal 四槽。
- 关键差异：glossiness 是"感知"值，转粗糙度时做 `RoughnessChannel = 1.0 - glossiness`（[sg.rs:220](../../scene/rendering/gpu-indirect/src/material/sg.rs#L220)）；specular_glossiness 纹理的 rgb 乘 specular、w 乘 glossiness。其余（法线、alpha、标签）与 MR 一致。

### 差异对照

| | Unlit | PbrMR | PbrSG |
| --- | --- | --- | --- |
| 参数槽 | color(Vec4) + alpha | base_color + metallic/roughness + emissive + normal_scale + alpha | albedo + specular + glossiness + emissive + normal_scale + alpha |
| 纹理槽数 | 1 | 4 | 4 |
| 颜色空间 | 组件 sRGB，写入时转线性 | 组件线性 | 组件线性 |
| 通道 | DefaultDisplay | Color/Emissive/Metallic/Roughness + DefaultDisplay | Color/Specular/Emissive/Roughness(1-glossiness) + DefaultDisplay |
| 标签 | UnlitMaterialTag | PbrMRMaterialTag + LightableSurfaceTag | PbrSGMaterialTag + LightableSurfaceTag |
| UV fallback | 注入零 UV | 无 | 无 |

## 纹理绑定系统与间接采样

### 系统选择与两种实现

`use_texture_system`（[gpu-base/src/texture/mod.rs:13](../../scene/rendering/gpu-base/src/texture/mod.rs#L13)）按 `GPUTextureBindingSystemType` 三选一：GLES 单绑定（传统逐绘制绑定，indirect 路径不可用）、`TexturePool`、`Bindless`。`get_suitable_texture_system_ty`（[mod.rs:239](../../scene/rendering/gpu-base/src/texture/mod.rs#L239)）在 indirect 模式下优先 bindless（需 GPU 支持 binding array + 非均匀索引且每 stage 容量够，见 [bindless.rs:3](../../content/texture/gpu-system/src/bindless.rs#L3)），否则回退纹理池。viewer 中的选择与装配见 [frame_all.rs:123](../../application/viewer-content/src/rendering/frame_all.rs#L123)。

纹理池（[pool.rs:366](../../content/texture/gpu-system/src/pool.rs#L366)）：所有 2D 纹理由 packer 打包进一个 2D array atlas（`TEXTURE_POOL_FORMAT = Rgba8Unorm`，[pool.rs:10](../../content/texture/gpu-system/src/pool.rs#L10)），每张纹理的"层号 + 尺寸 + 偏移"写进 `TexturePoolTextureMeta` 地址表，采样器参数（寻址模式、过滤器）写进采样器信息表。绑定系统整体作为三个绑定（atlas + 地址表 + 采样器表）参与渲染。由于 atlas 里的纹理没有硬件 mip 链与硬件采样器，`sample_texture2d_indirect`（[pool.rs:429](../../content/texture/gpu-system/src/pool.rs#L429)）在着色器里**手动**实现了寻址模式、双线性/三线性过滤与 mip level 计算（`load_texel_layer` 按层采样），并处理 sRGB → 线性转换（非 GL 后端纹理以 sRGB 格式存进 Rgba8Unorm，按 meta 标记转换）。

bindless（[bindless.rs:29](../../content/texture/gpu-system/src/bindless.rs#L29)）：texture 与 sampler 各一个 binding array，`sample_texture2d_indirect` 直接按句柄非均匀索引后 `texture.sample(sampler, uv)`——硬件原生采样与 mip 链，`compute_base_level` 是空实现（[bindless.rs:63](../../content/texture/gpu-system/src/bindless.rs#L63)）。

### 着色器侧间接采样

`indirect_sample_enabled`（[material/mod.rs:33](../../scene/rendering/gpu-indirect/src/material/mod.rs#L33)）：

- 展开 `TextureSamplerHandlePair`，`has_texture = texture_handle != u32::MAX`。
- `compute_base_level`：片元阶段用 UV 导数与纹理尺寸算 mip base level（[pool.rs:411](../../content/texture/gpu-system/src/pool.rs#L411)），非片元阶段强制 0（dpdx 不可用）。
- `select_branched`：有纹理走 `sample_texture2d_indirect`，无纹理返回零值。
- 外层 `indirect_sample` 再用 `has_texture.select(r, default_value)` 回退默认值（如白色），供"无纹理时用参数里的颜色"这类合成。

材质片元 shader 用 `binding.bind_by(self.storage).index(id)` 取参数——存储缓冲与着色器侧绑定都在材质 GPU 组件的 `setup_pass` / `build` 里成对出现（[mr.rs:139](../../scene/rendering/gpu-indirect/src/material/mr.rs#L139)），与组件模型的绑定契约一致。

## std_model 侧组装：材质 id 表与管线哈希

### 材质 id 表

`use_std_model_renderer`（[std_model.rs:304](../../scene/rendering/gpu-indirect/src/std_model.rs#L304)）维护 `SceneStdModelStorage { mesh, material, skin }` 元数据表（按 std model 实体分配索引）：

- `mesh` / `skin` 字段由 `StandardModelRefAttributesMeshEntity` / `StandardModelRefSkin` 外键变化映射（`map_u32_index_or_u32_max`）写入。
- `material` 字段由调用方传入的 `material_key` 写入（[std_model.rs:325](../../scene/rendering/gpu-indirect/src/std_model.rs#L325)）。viewer 里 `use_viewer_std_model_renderer`（[std_model_impl.rs:3](../../application/viewer-content/src/rendering/std_model_impl.rs#L3)）把四路材质外键（Unlit / PbrMR / PbrSG / Occ）的 `use_changes` 各自 `map_some_u32_index` 后 `SelectChanges` 合并——三类标准材质外键互斥，合并结果就是"这个 std model 的材质实体分配索引（或 None）"。
- 材质渲染器列表（`Vec<Box<dyn IndirectModelMaterialRenderImpl>>`）与形状渲染器列表由 [frame_all.rs:294](../../application/viewer-content/src/rendering/frame_all.rs#L294) 组装：`[unlit, pbr_mr, pbr_sg, occ]`。

### 顶点阶段注入

`SceneStdModelIdInjector`（[std_model.rs:410](../../scene/rendering/gpu-indirect/src/std_model.rs#L410)）在顶点阶段：

- 查询 `LogicalRenderEntityId`（绘制 id，即 id 池里的场景模型分配索引）→ 经 `sm_to_std_model_device` 缓冲（`SceneModelStdModelRenderPayload` 外键映射，[std_model.rs:311](../../scene/rendering/gpu-indirect/src/std_model.rs#L311)）得到 std model 索引。
- 读 `SceneStdModelStorage`，注册 `IndirectStdModelId`、`IndirectAbstractMaterialId`、`IndirectSkinId`、`IndirectAbstractMeshId` 四个语义，并把材质 id `set_vertex_out` 插值到片元阶段。
- 同时 `self.states.build(builder)` 应用光栅化状态覆盖（混合/深度/模板，[gpu-base/src/state.rs](../../scene/rendering/gpu-base/src/state.rs)，详见 [batch-extractor-guide.md](batch-extractor-guide.md)）。

### 绘制组合

`render_indirect_batch_models`（[scene_model.rs:96](../../scene/rendering/gpu-indirect/src/scene_model.rs#L96)）把六个组件包进一个 `RenderArray` 并分配绑定组：纹理系统独占组 0，draw provider / pass / camera 组 1，材质、形状、节点、model_info（含 midc 降级包装）组 2（[scene_model.rs:136](../../scene/rendering/gpu-indirect/src/scene_model.rs#L136)）。绘制命令由 draw provider 的 `draw_command()` 取出，以 `RenderMethod::TraditionalDraw` 提交。材质侧与 MIDC 降级无关（降级只改索引解包，见 [indirect-draw-command-guide.md](indirect-draw-command-guide.md)）。

### 管线哈希与分组 key 的一致性

`SceneStdModelIndirectRenderer::hash_shader_group_key`（[std_model.rs:386](../../scene/rendering/gpu-indirect/src/std_model.rs#L386)）把材质、形状、状态三路的 group key 汇合进同一个 PSO 哈希（代码片段见 [batch-extractor-guide.md](batch-extractor-guide.md) 的「从 group key 到 PSO：管线哈希的汇合点」）。

材质侧哈希的是 `alpha_mode` 与 `TypeId`（[mr.rs:83](../../scene/rendering/gpu-indirect/src/material/mr.rs#L83)），与材质 GPU 组件的 `hash_pipeline`（`alpha_mode` + `shader_hash_type_id!`，[mr.rs:132](../../scene/rendering/gpu-indirect/src/material/mr.rs#L132)）内容一致——group key 是"PSO 分桶"的廉价投影，管线哈希是真实管线缓存 key，两者必须严格对应（规则见 [fundamental-gpu-component-model-zh.md](skill-translation/fundamental-gpu-component-model-zh.md)）。批提取侧 `MaterialGroupKey::Common { ty, require_alpha_blend }`（见 [batch-extractor-guide.md](batch-extractor-guide.md)）从 `AlphaModeOf` 组件推得，与这里一致：alpha_mode 变化 → group key 变化 → 实体换桶；同桶内实体共享管线与间接绘制。

host-driven 路径（GLES 渲染后端但用间接绘制）直接用同一套 group key 逐实体分类：`classify_draws`（[scene.rs:16](../../scene/rendering/gpu-indirect/src/scene.rs#L16)）对每个场景模型算 `hash_shader_group_key_with_self_type_info`，按 hash 分桶后 `create_batch_from_iter` 现场构造 id 池与子列表（[scene.rs:42](../../scene/rendering/gpu-indirect/src/scene.rs#L42)）。

## 用户视角：创建材质、绑定纹理

viewer 中创建一个带纹理的 PBR MR 材质（模式详见 [viewer-scene-building-zh.md](skill-translation/viewer-scene-building-zh.md)）：

```rust
// 1. 纹理实体 + 采样器实体（Texture2DWithSamplingDataView 持有两个句柄）
let mut tw = writer.texture_sample_pair_writer();
let texture = tw.write_direct_tex_with_default_sampler(gpu_image); // 见 scene/core/src/texture.rs:138

// 2. 材质 DataView → 材质实体（组件写进数据库，纹理槽是双外键）
let material = PhysicalMetallicRoughnessMaterialDataView {
  base_color: Vec3::splat(0.8),
  base_color_texture: Some(texture),
  roughness: 0.1,
  metallic: 0.8,
  alpha: AlphaConfigDataView { alpha_mode: AlphaMode::Blend, ..Default::default() },
  ..Default::default()
}
.write(&mut writer.pbr_mr_mat_writer);            // scene/core/src/material.rs:276
let material = SceneMaterialDataView::PbrMRMaterial(material);

// 3. 挂到场景模型
let child = writer.create_root_child();
writer.set_local_matrix(child, Mat4::translate((0., 0., 0.)).into_f64());
writer.create_scene_model(material, mesh, child, scene);
```

之后无需任何渲染侧注册：材质组件与外键变化自动驱动稀疏写（上文"增量稀疏写"），纹理内容经 `viewer_texture_input`（[viewer-content/src/data_source.rs:344](../../application/viewer-content/src/data_source.rs#L344)，URI 或直接数据 → `GPUBufferImage`）进入纹理绑定系统。修改材质的 roughness、换纹理、改 alpha_mode 都会在下一帧生效：前两者只触发稀疏写（同管线），后者触发换桶 + 换管线。

## 与 host 路径（GLES）的对照

同一批材质实体在 [gpu-gles/src/material/](../../scene/rendering/gpu-gles/src/material/mod.rs) 有 uniform 版实现：同样的"组件变化 → 句柄映射 → 增量写"骨架（`use_tex_watcher` 的 uniform 变体，[gpu-gles/src/material/mod.rs:55](../../scene/rendering/gpu-gles/src/material/mod.rs#L55)），但数据落在 `UniformBufferCollection`（std140、按材质实体索引），绘制时逐实体 `make_component` 绑定（trait `GLESModelMaterialRenderImpl`，[mod.rs:75](../../scene/rendering/gpu-gles/src/material/mod.rs#L75)），纹理用 `setup_tex` 现场绑定句柄（[mod.rs:18](../../scene/rendering/gpu-gles/src/material/mod.rs#L18)）。两侧共享场景侧的纹理槽外键语义与 `TextureWithSamplingForeignKeys`，只换了"存储形态（storage array vs uniform collection）"与"采样路径（间接 vs 直接绑定）"。本条路径的完整展开（trait 体系、逐实体 uniform 维护、`setup_tex` 双句柄直绑、网格/节点/蒙皮/状态侧）见 [gles-material-host-render-guide.md](gles-material-host-render-guide.md)。

## 延伸阅读

- 几何侧（网格存储、两跳寻址、间接命令生成）：[attribute-mesh-indirect-render-guide.md](attribute-mesh-indirect-render-guide.md)
- 批提取与 group key（材质 alpha 语义如何分桶）：[batch-extractor-guide.md](batch-extractor-guide.md)
- 间接绘制命令与 MIDC 降级：[indirect-draw-command-guide.md](indirect-draw-command-guide.md)
- 纹理池打包与 atlas 更新：[content/texture/gpu-system/src/pool.rs](../../content/texture/gpu-system/src/pool.rs)、[gpu-base/src/texture/mod.rs:108](../../scene/rendering/gpu-base/src/texture/mod.rs#L108)
- 稀疏写入源与存储缓冲维护：[platform/graphics/webgpu-hook-utils/src/sparse_update_storage_buffer.rs](../../platform/graphics/webgpu-hook-utils/src/sparse_update_storage_buffer.rs)
- 光照系统对材质通道与标签的消费：[content/lighting/gpu-system/lighting-system/src/lib.rs:65](../../content/lighting/gpu-system/lighting-system/src/lib.rs#L65)
- RTX 路径复用同一批材质渲染器（`SceneMaterialSurfaceSupport`）：[scene/rendering/gpu-ray-tracing/src/material/mod.rs:7](../../scene/rendering/gpu-ray-tracing/src/material/mod.rs#L7)
