# Rendiation 间接绘制链路组织指南（scene/rendering/gpu-indirect + scheduler）

本文梳理间接渲染（indirect rendering）的「链路组织」：从场景数据库出发，批提取把实体按 PSO key 分桶成 GPU id 池（[batch-extractor-guide.md](batch-extractor-guide.md) 已覆盖），GPU 剔除（frustum / 遮挡）把 id 流压紧（[draw-list-guide.md](draw-list-guide.md)、[occlusion-culling-guide.md](occlusion-culling-guide.md) 已覆盖），mid 层把 id 流变成间接绘制命令（[indirect-draw-command-guide.md](indirect-draw-command-guide.md) 已覆盖）——这些子系统各自有专属文档，本文聚焦它们**如何被组织进一帧**：

- [scene/rendering/gpu-indirect/src/scene.rs](../../scene/rendering/gpu-indirect/src/scene.rs) 的 `use_make_scene_batch_pass_content`：`SceneRenderer` 的统一入口，把「一份批」变成「可直接放进 pass 的间接绘制内容（provider 列表）」。它回答了三个问题：批里的子列表各属于哪个渲染实现、每个实现的命令怎么生成、生成的 provider 怎么和子列表一一对应。
- [scene/rendering/scheduler/src/lib.rs](../../scene/rendering/scheduler/src/lib.rs) 的 `RenderBatchCollector`：帧末的「实际被绘制的批次」反馈接口，遮挡剔除把两遍绘制的最终批次喂给它，拾取、调试等子系统可通过它消费「这一帧到底画了谁」。
- 二者之间的整条帧内链路如何在 [frame_all.rs](../../application/viewer-content/src/rendering/frame_all.rs) 与 [light_pass/mod.rs](../../application/viewer-content/src/rendering/lighting/light_pass/mod.rs) 组装。

## 前置阅读

链路组织建立在批提取、GPU 绘制列表、遮挡剔除与渲染帧组装之上，建议先了解：

| 文档 | 内容 |
| --- | --- |
| [skill-translation/frame-pass-assemble-zh.md](skill-translation/frame-pass-assemble-zh.md) | FrameCtx、pass 组装、`PassContent`、scope 与 keyed_scope 的资源生命周期 |
| [skill-translation/query-system-zh.md](skill-translation/query-system-zh.md) | DualQuery 增量模型（批提取与剔除的增量输入来源） |
| [skill-translation/fundamental-gpu-component-model-zh.md](skill-translation/fundamental-gpu-component-model-zh.md) | ShaderHashProvider / ShaderPassBuilder / RenderComponent 与管线哈希 |
| [skill-translation/shader-edsl-compute-zh.md](skill-translation/shader-edsl-compute-zh.md) | compute 管线构建与间接派发（命令生成、子列表重算都靠它） |
| [skill-translation/scene-core-structure-zh.md](skill-translation/scene-core-structure-zh.md) | SceneModelEntity / StandardModelEntity、payload 外键、节点可见性 |
| [batch-extractor-guide.md](batch-extractor-guide.md) | 增量 PSO key 与 id 池：`DeviceSceneModelDrawList` 如何按 key 分桶（本链路的输入） |
| [draw-list-guide.md](draw-list-guide.md) | DeviceDrawList、`AbstractCullerProvider` 剔除抽象、流压缩，以及 frustum 剔除的 `GPUFrustumCuller`（frustum 无独立 guide，见其下游消费章节） |
| [indirect-draw-command-guide.md](indirect-draw-command-guide.md) | mid 层：`IndirectDrawProvider` / `DrawCommandBuilder` / MIDC 降级 |
| [occlusion-culling-guide.md](occlusion-culling-guide.md) | 两遍遮挡剔除与 `GPUTwoPassOcclusionCullingResult` |
| [attribute-mesh-indirect-render-guide.md](attribute-mesh-indirect-render-guide.md) | 属性网格作为「形状实现」对 provider / builder 的完整实现 |

## 模式概览

间接渲染链路分两段：**常驻增量段**（帧间不重算）与**每帧组织段**（帧内装配）。本文的两个主角都在每帧组织段：

- **`use_make_scene_batch_pass_content` 是批与 pass 之间的唯一桥梁**。所有「把场景模型画进某个 pass」的调用方（光照 pass、遮挡剔除的两遍、widget 场景、调试视图）都走它。它不关心批怎么来的（增量提取器或 host 全量提取），只做三件事：把批的子列表按「渲染实现」分类、为每个实现创建间接绘制 provider（生成命令 buffer + count view）、把 provider 与代表实体配对成 pass content。
- **每帧分类是必需的**。id 池按 PSO key 分桶，但「PSO key 相同」不等于「实现相同」——标准网格、单元网格、宽线、文字各有实现。子列表的共享管线只保证「一个子列表一个实现」，不同子列表可以分属不同实现，所以帧内要按实现二次分组，每个实现拿到自己的子列表子集后独立生成命令。
- **`RenderBatchCollector` 是帧末的「实际绘制批次」出口**。遮挡剔除在 `generate_culling_result` 开启时保留两遍绘制的最终批（`drawn_not_occluded` 与 `drawn_occluder`），帧末经 `feedback_culling_result` 喂给 collector；collector 的 `will_collecting` 反过来在帧前决定遮挡剔除是否保留结果。当前代码库里唯一的实现是 `DoNothingRenderBatchCollector`（默认安全），它是为拾取、调试等子系统预留的注入点。
- **两条绘制路径共享同一入口**。device 路径（id 池 + GPU 剔除 + GPU 命令生成）与 host-driven 路径（CPU 遍历实体、现场生成间接命令，用于 GL 后端）都在 `use_make_scene_batch_pass_content` 内分派，靠 `using_host_driven_indirect_draw` 与 `SceneModelRenderBatch::Host` 组合选择。

## 核心概念

| 概念 | 定义位置 | 说明 |
| --- | --- | --- |
| `SceneRenderer` | [scene/rendering/gpu-base/src/lib.rs:104](../../scene/rendering/gpu-base/src/lib.rs#L104) | 场景渲染能力 trait：`use_make_scene_batch_pass_content` + `indirect_batch_direct_creator` |
| `SceneRendererPassContentSource` | [gpu-base/src/lib.rs:95](../../scene/rendering/gpu-base/src/lib.rs#L95) | 批 → `PassContent` 的中间体：`as_pass_content(camera, pass)` 需要相机与 pass 组件 |
| `SceneDeviceBatchDirectCreator` | [gpu-base/src/lib.rs:118](../../scene/rendering/gpu-base/src/lib.rs#L118) | host 实体流 → `DeviceSceneModelDrawList` 的转换入口（现场建池路径） |
| `SceneModelRenderBatch` | [gpu-base/src/batch.rs:7](../../scene/rendering/gpu-base/src/batch.rs#L7) | 批的两种形态：`Device(Option<DeviceSceneModelDrawList>)` / `Host(Box<dyn HostRenderBatch>)` |
| `DeviceSceneModelDrawList` | [gpu-base/src/batch.rs:30](../../scene/rendering/gpu-base/src/batch.rs#L30) | 设备批：`DeviceDrawList`（id 池 + 子列表范围）+ `impl_select_ids`（每子列表代表实体） |
| `use_culled_list_and_do_culling` | [shader/draw-list/src/stream_compact/mod.rs:11](../../shader/draw-list/src/stream_compact/mod.rs#L11) | 用 `AbstractCullerProvider` 对批做剔除 + 流压缩，产出新批（范围 count 被压紧） |
| `IndirectSceneRenderer` | [gpu-indirect/src/scene.rs:3](../../scene/rendering/gpu-indirect/src/scene.rs#L3) | 间接渲染器：实现 `SceneRenderer`、`IndirectDrawProviderCreator`、`SceneDeviceBatchDirectCreator` |
| `IndirectDrawProviderCreator` | [gpu-indirect/src/scene.rs:114](../../scene/rendering/gpu-indirect/src/scene.rs#L114) | 按代表实体选实现、为子列表子集创建 `IndirectDrawProvider` 的 trait |
| `DrawCommandBuilderCreator` | [gpu-indirect/src/scene.rs:127](../../scene/rendering/gpu-indirect/src/scene.rs#L127) | 按代表实体取 `DrawCommandBuilder`（host-driven 路径用） |
| `use_compute_selected_sub_list_dispatch_info` | [gpu-indirect/src/scene.rs:270](../../scene/rendering/gpu-indirect/src/scene.rs#L270) | 为「选中子列表子集」重算两套 `MultiRangeDispatchInfo`（原始池偏移 / 紧凑偏移） |
| `IndirectScenePassContentSource` / `IndirectScenePassContent` | [gpu-indirect/src/scene.rs:413](../../scene/rendering/gpu-indirect/src/scene.rs#L413)、[:436](../../scene/rendering/gpu-indirect/src/scene.rs#L436) | 批 → pass content 的实现：`Vec<(Box<dyn IndirectDrawProvider>, SceneModelEntity)>` |
| `IndirectBatchSceneModelRenderer` | [gpu-indirect/src/scene_model.rs:17](../../scene/rendering/gpu-indirect/src/scene_model.rs#L17) | 真正的渲染执行者：把 provider、节点、形状、材质、相机组装成一个 `RenderComponent` 数组绘制 |
| `IndirectModelRenderImpl` | [gpu-indirect/src/std_model.rs:5](../../scene/rendering/gpu-indirect/src/std_model.rs#L5) | 渲染实现的抽象：hash 组 key、id 注入、形状/材质组件、索引 buffer 访问 |
| `HostDrivenIndirectProvider` | [gpu-indirect/src/host_driven.rs:98](../../scene/rendering/gpu-indirect/src/host_driven.rs#L98) | host-driven 路径的 provider：`DowngradeMultiIndirectDrawCountHelper` + 现场生成的命令 |
| `RenderBatchCollector` | [scheduler/src/lib.rs:11](../../scene/rendering/scheduler/src/lib.rs#L11) | 帧末收集「实际绘制批次」的 trait：`is_collecting` / `will_collecting` / `collect_batch` / `flush_frame` |
| `DoNothingRenderBatchCollector` | [scheduler/src/lib.rs:18](../../scene/rendering/scheduler/src/lib.rs#L18) | 默认空实现（不收集），viewer 的默认值 |
| `BasicScheduler` | [scheduler/src/lib.rs:33](../../scene/rendering/scheduler/src/lib.rs#L33) | 同 crate 的权重有序调度器（BTreeSet + 反馈更新），独立于批量收集 |
| `ViewerCulling` / `ViewerOcclusionCulling` | [application/viewer-content/src/rendering/culling.rs:75](../../application/viewer-content/src/rendering/culling.rs#L75)、[:65](../../application/viewer-content/src/rendering/culling.rs#L65) | viewer 侧剔除封装：frustum 执行、OC 状态持有、剔除结果反馈 |
| `GPUTwoPassOcclusionCullingResult` | [scene/rendering/occlusion-culling/src/lib.rs:198](../../scene/rendering/occlusion-culling/src/lib.rs#L198) | OC 的调试/采集结果：`drawn_occluder` 与 `drawn_not_occluded` 两个批 |
| `ViewerBatchExtractor` | [application/viewer-content/src/rendering/frame_all.rs:713](../../application/viewer-content/src/rendering/frame_all.rs#L713) | device（增量）与 host（全量）提取器之间的选择器 |

## 分层动机与数据流

先看一帧内完整链路，再逐层展开：

```text
场景数据库（实体 / 组件 / 外键）
  │
  ├─ 增量常驻段（帧间不重算，batch-extractor-guide 覆盖）
  │    use_scene_model_group_key_with_scene_id_and_visible_filter
  │      └─ (SceneModelGroupKey, scene_id) 增量查询
  │           └─ IncrementalDeviceSceneBatchExtractor（宿主列表 + GPU id 池，按 key 分桶）
  │                └─ extract_scene_batch(scene, SceneContentKey)
  │                     └─ SceneModelRenderBatch::Device(DeviceSceneModelDrawList)
  │                          └─ 子列表范围 + impl_select_ids（每子列表代表实体）
  │
  └─ 每帧组织段（本文主体）
       │
       ├─ light_pass：extract opaque / transparent 两批
       │    └─ ViewerCulling::use_draw_with_oc_maybe_enabled
       │         ├─ frustum 剔除（未启用 OC 时）：
       │         │    use_culled_list_and_do_culling(GPUFrustumCuller) → 新批
       │         └─ 遮挡剔除（启用 OC 时，occlusion-culling-guide 覆盖细节）：
       │              GPUTwoPassOcclusionCulling::use_draw
       │                ├─ 拆「上帧可见 / 不可见」两批
       │                ├─ 第一遍：use_make_scene_batch_pass_content(first_pass_batch)
       │                │    └─ 画 occluder + 生成深度金字塔 + 遮挡测试回写状态
       │                └─ 第二遍：use_make_scene_batch_pass_content(second_pass_batch)
       │                     └─ 画「未被遮挡」的实体
       │              └─ generate_culling_result 时保留 GPUTwoPassOcclusionCullingResult
       │
       └─ use_make_scene_batch_pass_content（SceneRenderer 入口）
            ├─ Device(Some(list))：按 impl_select_ids 的 impl key 二次分类子列表
            │    └─ 每类：use_compute_selected_sub_list_dispatch_info（子集重算范围）
            │         └─ use_create_or_update_indirect_draw_providers
            │              └─ mid：compute 生成间接命令 → 每子列表一个 IndirectDrawProvider
            │                   ├─ MultiIndirectDrawBatch（原生 MIDC）
            │                   └─ MIDCDowngradeBatch（降级，webgpu-midc-downgrade）
            │    └─ 配对 (provider, 代表实体) → IndirectScenePassContent
            │         └─ 渲染：render_indirect_batch_models 组装全部 RenderComponent
            │              └─ TraditionalDraw(provider.draw_command())
            ├─ Host(batch) + using_host_driven_indirect_draw：
            │    └─ process_host_driven_indirect_draws（CPU 遍历 → 现场生成命令）
            └─ Device(None)：空 pass content

帧末：
  batch_collector.is_collecting()
    └─ ViewerCulling::feedback_culling_result(collector)
         └─ collect_batch(drawn_not_occluded) + collect_batch(drawn_occluder)
              └─ collector.flush_frame()
```

分层动机：

- **常驻增量与每帧组织分离**。id 池与 key 分桶只在数据变化时增量维护；每帧只做「选子列表、生成命令、画」，成本与帧率解耦。
- **批与实现解耦**。提取器按 PSO key 分桶，不关心实现；渲染器按实现再分类，不关心 key 的细节。扩展新模型类型只需在 key 层（`GroupKeyForeignImpl`）与实现层（`IndirectModelRenderImpl`）各自接入。
- **一个入口，所有画法**。剔除（frustum / OC 两遍）、光照、widget、调试视图全部通过 `use_make_scene_batch_pass_content` 画场景模型，命令生成逻辑只存在一处。
- **collector 是可选的**。收集「实际绘制批次」有代价（OC 必须保留结果、帧末要遍历），所以用 `will_collecting` 帧前询问、`is_collecting` 帧末决定是否反馈，不收集时零开销。

## 帧内链路总览（用户视角）

### 装配：use_viewer_scene_renderer

[frame_all.rs](../../application/viewer-content/src/rendering/frame_all.rs) 的 `use_viewer_scene_renderer`（[:100](../../application/viewer-content/src/rendering/frame_all.rs#L100)）按后端类型装配：

- **Indirect 分支**（[:233](../../application/viewer-content/src/rendering/frame_all.rs#L233)）：依次建立材质存储、节点存储、LOD 网格、宽线/宽点/文字渲染器，`use_indirect_scene_model` 把 `Vec<Box<dyn IndirectModelRenderImpl>>`（std model + 宽线 + 宽点 + 文字 + 实例化模型）包成 `IndirectPreferredComOrderRenderer`，最后 `cx.when_render` 产出 `IndirectSceneRenderer`（[:423](../../application/viewer-content/src/rendering/frame_all.rs#L423)）。
- **批提取器**：非 host-driven 时在同一 scope 内组装增量 key（`use_scene_model_group_key` + occ layer + 实例化 key，[:385](../../application/viewer-content/src/rendering/frame_all.rs#L385)），用 `use_occ_incremental_device_scene_batch_extractor` 建 `indirect_extractor`（[:419](../../application/viewer-content/src/rendering/frame_all.rs#L419)）；host 侧总有一个 `use_occ_host_scene_batch_extractor`（[:502](../../application/viewer-content/src/rendering/frame_all.rs#L502)）。`ViewerBatchExtractor`（[:713](../../application/viewer-content/src/rendering/frame_all.rs#L713)）优先用 indirect_extractor，否则回退 host。
- **剔除**：`use_viewer_culling`（[culling.rs:6](../../application/viewer-content/src/rendering/culling.rs#L6)）只在 indirect 且启用 OC 时创建 `GPUTwoPassOcclusionCulling` 状态（每相机一个），frustum 用 `use_camera_gpu_frustum`（[frustum-culling/src/lib.rs:15](../../scene/rendering/frustum-culling/src/lib.rs#L15)）上传相机视锥。

### 提取与绘制：use_render_lighting_scene_content

[light_pass/mod.rs](../../application/viewer-content/src/rendering/lighting/light_pass/mod.rs) 的 `use_render_lighting_scene_content` 是场景内容真正上屏的地方：

- 用 `renderer.batch_extractor.extract_scene_batch` 提取两批：不透明（[:53](../../application/viewer-content/src/rendering/lighting/light_pass/mod.rs#L53)，`SceneContentKey::only_opaque_objects()`）与透明（[:69](../../application/viewer-content/src/rendering/lighting/light_pass/mod.rs#L69)，`only_alpha_blend_objects()`）。透明过滤直接落在提取器的 key 上（`require_alpha_blend`，见 [batch-extractor-guide.md](batch-extractor-guide.md) 的材质侧 key）。
- 不透明批交给 `ViewerCulling::use_draw_with_oc_maybe_enabled`（[culling.rs:141](../../application/viewer-content/src/rendering/culling.rs#L141)）：启用 OC 时跳过 frustum（[:159](../../application/viewer-content/src/rendering/culling.rs#L159) 的注释说明 OC 已覆盖视锥外对象），批进入 `GPUTwoPassOcclusionCulling::use_draw`；否则先 `use_execute_frustum_culler`（[:103](../../application/viewer-content/src/rendering/culling.rs#L103)）再直接 `use_make_scene_batch_pass_content`。
- 透明批由 `transparent_content_renderer.use_render` 处理（Forward 模式下走同一剔除入口，Defer 模式在光照计算后的单独 forward pass 里画，见 [light_pass/mod.rs:234](../../application/viewer-content/src/rendering/lighting/light_pass/mod.rs#L234)）。widget 场景（如坐标轴）同样用「extract + use_make_scene_batch_pass_content」绘制（[application/viewer/src/viewer/widget/mod.rs:53](../../application/viewer/src/viewer/widget/mod.rs#L53)）。

### 帧驱动：render 与渲染实例

帧末驱动在 [frame_all.rs:607](../../application/viewer-content/src/rendering/frame_all.rs#L607) 的 `render`：

- 先 `lighting.prepare` 准备光照上下文（光照系统也是 `SceneRenderer` 的消费者，见 [lighting/mod.rs:108](../../application/viewer-content/src/rendering/lighting/mod.rs#L108) 的批提取）。
- `set_should_keep_oc_cull_result(will_collecting())` 提前告知剔除系统本帧是否保留结果。
- 对每个请求的 viewport `ctx.keyed_scope(viewport_id)`：设置帧尺寸、相机 uniform、LOD 控制，然后 `view_renderer.use_render` 走 [light_pass/mod.rs](light_pass/mod.rs) 的完整 pass 序列（背景、场景不透明/透明、clip、后处理）。所有资源创建都在 `FrameCtx` 的 scope 内，按 viewport 缓存（`Viewer3dViewportRenderingCtx`，[frame_all.rs:582](../../application/viewer-content/src/rendering/frame_all.rs#L582)）。
- 帧末按 `is_collecting()` 决定是否 feedback 与 flush（[:689](../../application/viewer-content/src/rendering/frame_all.rs#L689)）。

`ViewerRendererInstance`（[:698](../../application/viewer-content/src/rendering/frame_all.rs#L698)）把相机、背景、`raster_scene_renderer`、`batch_extractor`、`culling`、裁剪、变换查询聚合在一起——`use_make_scene_batch_pass_content` 的所有调用方（光照、OC、widget）都从它取 `batch_extractor` 与 `culling`。整个渲染实例分两阶段由 `use_viewer_scene_renderer` 产出（[rendering_root.rs:147](../../application/viewer-content/src/rendering/rendering_root.rs#L147) 的 `Update` 阶段与 [:177](../../application/viewer-content/src/rendering/rendering_root.rs#L177) 的 `CreateRender` 阶段）：先跑增量维护（提取器、剔除状态、网格池的增量更新），再在帧编码前组装渲染实例——常驻增量段与每帧组织段的时序边界就在这里。

## use_make_scene_batch_pass_content 的实现

### 入口与分支

[scene.rs:167](../../scene/rendering/gpu-indirect/src/scene.rs#L167)。注意入口处的 `ctx.next_scope_index()` 与内部的 `ctx.scope` / `ctx.keyed_scope(impl_key)` / `ctx.access_parallel_compute`：`FrameCtx` 的 scope 约定决定所有 compute 资源（命令池、count buffer、范围 buffer）的生命周期——同一 scope 内跨帧复用，scope 变化则重建（详见 [skill-translation/frame-pass-assemble-zh.md](skill-translation/frame-pass-assemble-zh.md)）。逐实现 `keyed_scope` 使每个实现组的命令生成资源按实现 key 缓存，不随子列表增减抖动。

分支逻辑：

- `Device(batch)`：直接使用设备批。
- `Host(batch)` 且 `using_host_driven_indirect_draw`：直接返回 host-driven 内容（见下文 host-driven 分支）。
- `Host(batch)` 且非 host-driven：调用 `create_batch_from_iter`（[:44](../../scene/rendering/gpu-indirect/src/scene.rs#L44)）现场建批——`classify_draws` 按 `PipelineHasher` + `hash_shader_group_key_with_self_type_info` 把实体流分成实现组，按组对齐填充 id 池（`round_up` 到 storage offset 对齐），`prepare_gpu_sub_list_ranges` 生成范围并组装 `DeviceDrawList`。hash 失败（如网格未加载）的实体经 `SceneModelErrorRecorder` 记录并过滤（[:29](../../scene/rendering/gpu-indirect/src/scene.rs#L29)）。
- `Device(None)`：空批，返回空 content（GPU 层不允许零长 buffer，`None` 是约定）。

注意 `indirect_batch_direct_creator`（[:159](../../scene/rendering/gpu-indirect/src/scene.rs#L159)）在 host-driven 模式下返回 `None`，使 host 提取器（[batch_extraction.rs:100](../../scene/rendering/gpu-base/src/batch_extraction.rs#L100)）产出 `Host` 批而不是现场转 device 批——两种路径互斥，由 `using_host_driven_indirect_draw` 统一决定。

### 子列表 → 实现：二次分类

`DeviceSceneModelDrawList` 的每个子列表有一个 `impl_select_ids` 代表实体（提取器取组内第一个）。分类（[:191](../../scene/rendering/gpu-indirect/src/scene.rs#L191)）：

- 对每个子列表，用 `get_impl_distinguish_key_by_impl_select_id`（[:138](../../scene/rendering/gpu-indirect/src/scene.rs#L138)）取实现的区分 key，按 key 归组，同时记录 `(impl_key, 组内下标, 代表实体)` 三元组。
- 分类的合法性前提写在 trait 注释里（[:117](../../scene/rendering/gpu-indirect/src/scene.rs#L117)）：同一子列表内所有实体的 impl key 必须一致——因为子列表共享 PSO，PSO 哈希里含实现类型（`hash_shader_group_key_with_self_type_info` 会 `hasher.hash(type_id)`，见 [std_model.rs:11](../../scene/rendering/gpu-indirect/src/std_model.rs#L11)），所以「PSO 分桶」隐含「实现一致」。不同子列表仍可分属不同实现（标准网格 vs 单元网格 vs 宽线），因此需要按 key 二次分组。

### 子集重算 dispatch info

每个实现组拿到的是「若干子列表下标」。`use_compute_selected_sub_list_dispatch_info`（[:270](../../scene/rendering/gpu-indirect/src/scene.rs#L270)）把这份子集变成可派发的独立 `MultiRangeDispatchInfo`：一个 compute pass（每子列表一线程）读原始 `sub_list_ranges`，产出**两套**输出：

- `origin`：保留原始池偏移，供 `scene_model_id_pool` 索引（id 池地址不变）。
- `compacted`：按选中子列表的容量重新累计紧凑偏移，供 MIDC 降级的命令池切片（[indirect-draw-command-guide.md](indirect-draw-command-guide.md) 的 `MIDCListPoolInput`）。

GPU 同时把真实总数写进 `sum_all_count`（host 侧只用容量上界）。这与提取器的「容量对齐」约定（见 [batch-extractor-guide.md](batch-extractor-guide.md) 的 `CapacityRange`）配合：容量是上界，count 由 GPU 运行时决定。

### provider 创建与 content 组装

- 对每个实现组 `ctx.keyed_scope(impl_key)` 内调用 `use_create_or_update_indirect_draw_providers`（trait 在 [:118](../../scene/rendering/gpu-indirect/src/scene.rs#L118)）。std model 路径把它转发给 shape 实现（[std_model.rs:360](../../scene/rendering/gpu-indirect/src/std_model.rs#L360)），shape 实现（属性网格、单元网格）内部调用 mid 的 `use_and_create_default_indirect_draw_provider`（[gpu-base/src/mid/mod.rs:84](../../scene/rendering/gpu-base/src/mid/mod.rs#L84)）：compute 逐实体生成 `DrawIndexedIndirectArgsStorage` / `DrawIndirectArgsStorage` 到 `INDIRECT` buffer，按子列表切片成每子列表一个 provider（原生 `MultiIndirectDrawBatch`（[:318](../../scene/rendering/gpu-base/src/mid/mod.rs#L318)）或降级 `MIDCDowngradeBatch`）。每个 provider 持有自己的命令 buffer view + count view，数量与选中子列表一一对应。
- 返回的 provider 列表按下标放进 `FastHashMap<usize, Box<dyn IndirectDrawProvider>>`（用 map 避免 trait object 需要 Clone，[:243](../../scene/rendering/gpu-indirect/src/scene.rs#L243)）。
- 最后按分类时记录的三元组，把每个 provider 与它所属子列表的代表实体配对成 `content: Vec<(Box<dyn IndirectDrawProvider>, EntityHandle<SceneModelEntity>)>`（[:253](../../scene/rendering/gpu-indirect/src/scene.rs#L253)）。

provider 的数量等于选中子列表数而不是实体数：同一个实现的所有子列表共用一个命令池，每个子列表用不同的 buffer view 切片——这正是一开始「PSO 分桶」的回报：命令生成的粒度是子列表，不是实体。

### 渲染：IndirectScenePassContent

`as_pass_content(camera, pass)`（[:422](../../scene/rendering/gpu-indirect/src/scene.rs#L422)）把内容源与相机、pass 组件绑定；`PassContent::render`（[:447](../../scene/rendering/gpu-indirect/src/scene.rs#L447)）对每个 (provider, 代表实体) 调用 `render_indirect_batch_models`：

- 基础 dispatcher：`default_dispatcher(cx, reversed_depth).disable_auto_write()`（来自 [webgpu/src/frame/pass_base.rs:3](../../platform/graphics/webgpu/src/frame/pass_base.rs#L3)），与 pass 组件合成后传给每个实体。
- [scene_model.rs:96](../../scene/rendering/gpu-indirect/src/scene_model.rs#L96) 的组装：`draw_source`（`IndirectDrawProviderAsRenderComponent`，顶点阶段注册 `LogicalRenderEntityId`，见 [mid/mod.rs:43](../../scene/rendering/gpu-base/src/mid/mod.rs#L43)）+ 纹理系统 + pass + MIDC 降级包装 + `model_info_injector` + shape + node + camera + material，按绑定索引拼成 `RenderArray`（[:136](../../scene/rendering/gpu-indirect/src/scene_model.rs#L136)），用 `TraditionalDraw(provider.draw_command())` 提交。
- `model_info_injector`（[std_model.rs:406](../../scene/rendering/gpu-indirect/src/std_model.rs#L406)）是标准模型的「id 注入」：顶点阶段用 `LogicalRenderEntityId`（当前实例的实体 id）查 `sm_to_std_model_device` 得到 std model 索引，再从 `SceneStdModelStorage`（[:487](../../scene/rendering/gpu-indirect/src/std_model.rs#L487)，mesh/material/skin 三个 u32）解出材质、网格、蒙皮 id 注册成语义（`IndirectAbstractMaterialId` / `IndirectAbstractMeshId` / `IndirectSkinId`），材质与形状组件据此取数。这份元数据由 `use_std_model_renderer`（[:304](../../scene/rendering/gpu-indirect/src/std_model.rs#L304)）用增量查询逐字段维护（`update_storage_array` 把网格/蒙皮外键、材质 key、状态覆盖写进 `std_model` 存储 buffer，`use_max_item_count_by_db_entity` 按数据库实体数决定容量）——宿主侧的结构化数据以「表」的形式镜像到 GPU，顶点阶段按需解引用，这正是「一个实体 → 一跳映射 → 元数据行」的两跳间接寻址模式。

### 错误处理与降级

链路里「一个实体画不出来」是常态（网格未加载、实现不匹配），各环节的约定是：**能跳过就跳过，绝不画错**。

- `classify_draws` 里 `hash_shader_group_key_with_self_type_info` 返回 `None` 的实体（如网格数据未就绪）经 `SceneModelErrorRecorder::report_and_filter_error` 记录后不进任何组（[scene.rs:29](../../scene/rendering/gpu-indirect/src/scene.rs#L29)）。
- 子列表分类找不到实现 key（`get_impl_distinguish_key_by_impl_select_id` 返回 `None`）时记 `log::error`，该子列表不参与绘制（[scene.rs:213](../../scene/rendering/gpu-indirect/src/scene.rs#L213)）；`use_create_or_update_indirect_draw_providers` 失败同理（[:247](../../scene/rendering/gpu-indirect/src/scene.rs#L247)）。这两处错误理论上不会触发——PSO 分桶与实现选择由同一份 key 保证一致，出错说明扩展接入时破坏了约定。
- 命令生成层面（mid）的约定是「空 drawcall 而非跳过」：网格未分配时生成 count 为零的命令，避免一次流压缩（见 [indirect-draw-command-guide.md](indirect-draw-command-guide.md) 的模式概览）。

## 剔除如何衔接

### frustum

`GPUFrustumCuller`（[frustum-culling/src/lib.rs:72](../../scene/rendering/frustum-culling/src/lib.rs#L72)）实现 `AbstractCullerProvider`：每个实体取世界 AABB，与 6 平面逐一做 `aabb_half_space_intersect`，任一平面完全在外即剔除。它通过 `use_culled_list_and_do_culling` 作用于 `DeviceSceneModelDrawList`（[batch.rs:37](../../scene/rendering/gpu-base/src/batch.rs#L37)），结果仍是同结构批（范围 count 被流压缩压紧），`impl_select_ids` 原样保留——所以剔除结果可以直接再进 `use_make_scene_batch_pass_content`。host 路径对应 `HostFrustumCulling`（[:149](../../scene/rendering/frustum-culling/src/lib.rs#L149)），在迭代器上过滤。viewer 中启用 OC 时跳过 frustum（[culling.rs:159](../../application/viewer-content/src/rendering/culling.rs#L159) 的注释说明 OC 做了同样的工作）。

### occlusion culling 两遍都走同一入口

[occlusion-culling/src/lib.rs:45](../../scene/rendering/occlusion-culling/src/lib.rs#L45) 的 `use_draw`：把输入批拆成「上帧可见 / 不可见」后，第一遍（[:98](../../scene/rendering/occlusion-culling/src/lib.rs#L98)）与第二遍（[:171](../../scene/rendering/occlusion-culling/src/lib.rs#L171)）分别以 `SceneModelRenderBatch::Device(Some(batch))` 调用 `scene_renderer.use_make_scene_batch_pass_content`——所以 OC 内部的两次绘制与普通光照绘制共享同一套 provider 创建逻辑。`generate_culling_result` 为真时才构造 `GPUTwoPassOcclusionCullingResult`（[:184](../../scene/rendering/occlusion-culling/src/lib.rs#L184)），其中 `drawn_not_occluded` 是第二遍真正画出的批。

`use_draw_with_oc_maybe_enabled`（[culling.rs:141](../../application/viewer-content/src/rendering/culling.rs#L141)）还处理了调试视图：viewport 配置了 `debug_camera_for_view_related` 时，把上一帧保留的 `drawn_occluder` / `drawn_not_occluded` 各自再走一遍 `use_make_scene_batch_pass_content` 画进调试窗口（[:165](../../application/viewer-content/src/rendering/culling.rs#L165)）——剔除结果本身就是「批」，可以直接复用整条渲染链路，这也是「批是链路通货」的最直观例证。OC 状态按相机持有（`oc_states: FastHashMap<SceneCameraEntity, Arc<RwLock<GPUTwoPassOcclusionCulling>>>`，[:66](../../application/viewer-content/src/rendering/culling.rs#L66)），多视图场景各相机互不干扰。

### feedback 与 collector

`ViewerCulling::feedback_culling_result`（[culling.rs:223](../../application/viewer-content/src/rendering/culling.rs#L223)）把每相机的 `drawn_not_occluded` 与 `drawn_occluder` 依次 `collect_batch` 给 collector。帧序（[frame_all.rs:607](../../application/viewer-content/src/rendering/frame_all.rs#L607) 的 `render`）：

- 帧前 `set_should_keep_oc_cull_result(batch_collector.will_collecting())`（[:633](../../application/viewer-content/src/rendering/frame_all.rs#L633)）——collector 想收集时，OC 本帧保留剔除结果（否则为性能直接丢弃）。
- 帧末 `is_collecting()` 为真才 `feedback_culling_result` + `flush_frame`（[:689](../../application/viewer-content/src/rendering/frame_all.rs#L689)）。

collector 的注入点是 `ViewerDataScheduler.batch_collector`（[data_source.rs:16](../../application/viewer-content/src/data_source.rs#L16)，默认 `DoNothingRenderBatchCollector`（[:73](../../application/viewer-content/src/data_source.rs#L73)），由 [rendering_root.rs:196](../../application/viewer-content/src/rendering_root.rs#L196) 每帧传入。拾取（picking）目前不消费渲染批：`ViewerPicker`（[pick.rs:3](../../application/viewer-content/src/pick.rs#L3)）基于 BVH + 光线/视锥查询（`SceneModelPicker`），与渲染链路解耦；`RenderBatchCollector` 是「每帧实际绘制批次」的预留出口（如绘制结果可视化、基于已绘批次的拾取加速）。

## host-driven 分支

`process_host_driven_indirect_draws`（[host_driven.rs:4](../../scene/rendering/gpu-indirect/src/host_driven.rs#L4)）用于 GL 后端（无 MIDC、无 GPU 流压缩能力）：CPU 每帧遍历实体（`classify_draws` 同样按实现分组），对每组取 `DrawCommandBuilder` 的 `draw_command_host_access` 现场生成 `DrawIndexedIndirectArgsStorage` / `DrawIndirectArgsStorage` 列表，经 `downgrade_multi_indirect_draw_count_host_driven` 组装成 `HostDrivenIndirectProvider`（[:98](../../scene/rendering/gpu-indirect/src/host_driven.rs#L98)）。它与 device 路径共用 `IndirectScenePassContentSource`，只是 provider 的实现不同——builder 的「host 访问 + compute 生成」双表达（[indirect-draw-command-guide.md](indirect-draw-command-guide.md) 的模式概览）正是为这一对称性设计的。

## 扩展点与使用模板

- **接入一个新的「实际绘制批次」消费者**：实现 `RenderBatchCollector`（`will_collecting` 返回 true 让 OC 保留结果，`is_collecting` 返回 true 使帧末反馈，`collect_batch` 内保存 `DeviceSceneModelDrawList` 克隆，`flush_frame` 做帧级收尾），替换 `ViewerDataScheduler.batch_collector`。
- **接入新的渲染实现**：实现 `IndirectModelRenderImpl`（组 key 哈希、`model_info_injector`、形状/材质组件），加入 `IndirectSceneRenderer.renderer` 的实现列表；key 层用 `GroupKeyForeignImpl` 分桶（见 [batch-extractor-guide.md](batch-extractor-guide.md) 的扩展机制）。`use_make_scene_batch_pass_content` 的分类自动把新实现的子列表路由到它的 provider 创建。
- **自建场景渲染器**：实现 `SceneRenderer` 后，OC、光照、widget 全部自动兼容——它们只依赖 `use_make_scene_batch_pass_content` 与 `indirect_batch_direct_creator` 两个接口。

## 阅读路线：从一帧到一行绘制

给第一次接触这条链路的新开发者一份「跟着代码走一遍」的路线：

- 起点：帧末驱动 [frame_all.rs:607](../../application/viewer-content/src/rendering/frame_all.rs#L607) 的 `render` → 每 viewport 的 `use_render` → [light_pass/mod.rs:20](../../application/viewer-content/src/rendering/lighting/light_pass/mod.rs#L20) 的 `use_render_lighting_scene_content`。
- 提取：`extract_scene_batch` 进 [gpu-base/src/batch_extraction.rs:21](../../scene/rendering/gpu-base/src/batch_extraction.rs#L21)（trait）→ 具体实现 `IncrementalDeviceSceneBatchExtractor::extract_scene_batch`（[batch-extractor/src/extractor.rs](../../scene/rendering/batch-extractor/src/extractor.rs)）→ 得到 `DeviceSceneModelDrawList`。
- 剔除：`use_draw_with_oc_maybe_enabled` → `use_execute_frustum_culler` 或 `GPUTwoPassOcclusionCulling::use_draw`，两者内部都经 `use_culled_list_and_do_culling`（[shader/draw-list/src/stream_compact/mod.rs:11](../../shader/draw-list/src/stream_compact/mod.rs#L11)）。
- 组织：`use_make_scene_batch_pass_content`（[scene.rs:167](../../scene/rendering/gpu-indirect/src/scene.rs#L167)）→ `use_compute_selected_sub_list_dispatch_info` → `use_create_or_update_indirect_draw_providers` → mid 的 `use_and_create_default_indirect_draw_provider`（[gpu-base/src/mid/mod.rs:84](../../scene/rendering/gpu-base/src/mid/mod.rs#L84)）——间接命令在这里生成。
- 绘制：`IndirectScenePassContent::render` → `render_indirect_batch_models`（[scene_model.rs:96](../../scene/rendering/gpu-indirect/src/scene_model.rs#L96)）→ `model_info_injector`（[std_model.rs:406](../../scene/rendering/gpu-indirect/src/std_model.rs#L406)）的顶点阶段 id 注入 → `TraditionalDraw(provider.draw_command())`。
- 帧末：`feedback_culling_result`（[culling.rs:223](../../application/viewer-content/src/rendering/culling.rs#L223)）→ `RenderBatchCollector`（[scheduler/src/lib.rs:11](../../scene/rendering/scheduler/src/lib.rs#L11)）。

想改「每帧画什么」改提取器与剔除；想改「画出来的命令长什么样」改 mid 的 builder；想改「绘制时绑定什么」改 `render_indirect_batch_models` 的组装与 `model_info_injector`；想新增「消费已画批次」的子系统，实现 `RenderBatchCollector` 挂到 `ViewerDataScheduler.batch_collector`。

## 常见疑问

- **为什么 provider 按子列表创建而不是按实体？** 子列表内的实体共享管线与命令池布局，命令生成 pass 一次覆盖整个子集（一线程一实体写命令池），每个子列表只需一条 `MultiIndirectDrawCount` 绘制（count 由 GPU 运行时的 `sum_all_count` 决定）。按实体创建会退化为逐实体提交，丢失间接绘制的意义。
- **为什么剔除结果可以直接复用 `use_make_scene_batch_pass_content`？** `use_culled_list_and_do_culling` 只压紧范围 count、不改变 id 池内容与子列表结构，`impl_select_ids` 原样保留——剔除前后的批是同构的，OC 两遍与调试视图因此可以直接复用同一入口。
- **为什么需要 origin 与 compacted 两套 dispatch info？** id 池索引必须用原始偏移（剔除不搬数据），而 MIDC 降级要把命令池按选中子列表切片，需要从 0 累计的紧凑偏移。一套数据两种视角，`use_compute_selected_sub_list_dispatch_info` 用一次 compute pass 同时产出。
- **host-driven 与 device 路径为什么互斥？** 两条路径维护同一份 GPU 数据（id 池 / 命令池）的不同方式：device 路径由增量提取器常驻维护，host-driven 路径每帧从数据库全量重建。同时启用会造成双写冲突，所以 `using_host_driven_indirect_draw` 同时关掉 `indirect_batch_direct_creator` 与增量提取器分支。
- **`RenderBatchCollector` 与 `BasicScheduler` 是什么关系？** 两者同在 [scheduler/src/lib.rs](../../scene/rendering/scheduler/src/lib.rs) 但没有依赖关系：前者是帧末绘制批次的反馈接口（本指南主题），后者是通用的权重有序调度器（`WeightOrdered` + 反馈更新 `iter_weights`，目前无消费方，注释标注「todo, consider move this to another crate」）。

## 延伸阅读

- 批提取与 id 池增量维护：[batch-extractor-guide.md](batch-extractor-guide.md)
- GPU 绘制列表、剔除抽象与流压缩：[draw-list-guide.md](draw-list-guide.md)
- 两遍遮挡剔除与帧间状态：[occlusion-culling-guide.md](occlusion-culling-guide.md)
- 间接命令生成与 MIDC 降级：[indirect-draw-command-guide.md](indirect-draw-command-guide.md)
- 属性网格的 provider / builder 实现：[attribute-mesh-indirect-render-guide.md](attribute-mesh-indirect-render-guide.md)
- 渲染帧组装与 FrameCtx：[skill-translation/frame-pass-assemble-zh.md](skill-translation/frame-pass-assemble-zh.md)
- 可组合 GPU 组件模型与管线哈希：[skill-translation/fundamental-gpu-component-model-zh.md](skill-translation/fundamental-gpu-component-model-zh.md)
