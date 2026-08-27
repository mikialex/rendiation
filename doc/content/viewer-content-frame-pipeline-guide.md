# Rendiation Viewer 帧流水线装配指南（application/viewer-content）

本文梳理 [application/viewer-content](../../application/viewer-content) 的渲染帧装配：`RenderingRoot`、`Viewer3dRenderingCtx`、`ViewerBatchExtractor` 三个核心对象如何被创建、维护与驱动，WebGPU（indirect）与 GLES 两条渲染路径在哪分流、各自动员哪些渲染器，以及一帧从窗口事件到屏幕上像素的完整走向。本文是 viewer 应用层的「帧组织」视角：[gpu-indirect-batch-collector-guide.md](gpu-indirect-batch-collector-guide.md) 聚焦间接绘制链路（批 → 剔除 → 间接命令）如何在帧内被调用，本文聚焦**谁持有谁、谁驱动谁、帧的骨架如何搭起来**——两条阅读线在 `use_viewer_scene_renderer` 与 `use_render_lighting_scene_content` 处交汇。

## 前置阅读

帧装配位于 viewer 应用层，它把数据库、query-hook 运行时、场景渲染子系统、GPU 帧组装串在一起。建议按序了解：

| 文档 | 内容 |
| --- | --- |
| [skill-translation/frame-pass-assemble-zh.md](skill-translation/frame-pass-assemble-zh.md) | FrameCtx、pass/attachment、scope 与 keyed_scope 的资源生命周期（理解帧骨架的语法基础） |
| [skill-translation/fundamental-gpu-component-model-zh.md](skill-translation/fundamental-gpu-component-model-zh.md) | RenderComponent / ShaderPassBuilder / 管线哈希（理解 pass 组件如何组合） |
| [query-hook-guide.md](query-hook-guide.md) | 两阶段执行模型（spawn/resolve）、共享计算、任务池与 waker（渲染器维护的驱动模型） |
| [hooks-guide.md](hooks-guide.md) | FunctionMemory 与状态生命周期（渲染进程内存分账的基础） |
| [skill-translation/scene-core-structure-zh.md](skill-translation/scene-core-structure-zh.md) | 场景实体类型与组件（帧里画的到底是什么） |
| [viewer-content-api-guide.md](viewer-content-api-guide.md) | 嵌入层 C API 如何使用本文的 `Viewer`（另一个宿主视角，可对照） |
| [batch-extractor-guide.md](batch-extractor-guide.md) | 批提取与增量 id 池（`ViewerBatchExtractor` 的两个实现之一） |
| [gpu-indirect-batch-collector-guide.md](gpu-indirect-batch-collector-guide.md) | 间接绘制链路在帧内的组织（`use_make_scene_batch_pass_content`、`RenderBatchCollector`，与本文重合的部分以它为准） |
| [occlusion-culling-guide.md](occlusion-culling-guide.md) | 两遍遮挡剔除的 GPU 内部机制（viewer 侧装配见本文「剔除的装配」） |
| [draw-list-guide.md](draw-list-guide.md) | DeviceDrawList、`AbstractCullerProvider` 与流压缩（frustum 剔除的底座） |
| [material-indirect-render-guide.md](material-indirect-render-guide.md) | 间接材质参数与纹理系统（Indirect 分支的材质侧） |
| [gles-material-host-render-guide.md](gles-material-host-render-guide.md) | GLES 材质与模型 host 渲染（Gles 分支的材质/模型/网格/节点实现） |

## 模式概览

viewer 应用层的帧组织可以浓缩为四句话：

- **三个常驻对象各管一段**。`RenderingRoot`（[rendering_root.rs](../../application/viewer-content/src/rendering_root.rs)）管帧驱动与帧资源池（attachment 池、pass 信息池、按 surface 分账的渲染进程内存）；`Viewer3dRenderingCtx`（[rendering/frame_all.rs](../../application/viewer-content/src/rendering/frame_all.rs)）管「渲染器的常驻维护」——所有 GPU 资源（纹理系统、剔除状态、材质/网格/模型存储、批提取器）都以 hook 状态的形式挂在它下面，帧间不销毁；`ViewerDataScheduler`（[data_source.rs](../../application/viewer-content/src/data_source.rs)）由宿主创建并注入 `DynCx`，管 URI 流式资源与帧末批收集器。三者都是 `Viewer`（[viewer.rs](../../application/viewer-content/src/viewer.rs)）的字段，`Viewer` 又是桌面应用与 C API 各自持有。
- **每帧两阶段驱动**。`RenderingRoot.draw_canvas` 用 `QueryGPUHookCx`（[platform/graphics/webgpu-hook-utils/src/hook.rs](../../platform/graphics/webgpu-hook-utils/src/hook.rs)）把 `use_viewer_scene_renderer` 执行两遍：`Update` 阶段做增量维护并派发异步任务，`CreateRender` 阶段在帧编码前组装出 `ViewerRendererInstance`——一份「本帧可用的渲染实例」；随后 `Viewer3dRenderingCtx::render` 逐视口编码全部 pass。常驻增量维护与每帧组织之间的时序边界就在这里。
- **双后端在同一个函数里分流**。`use_viewer_scene_renderer` 按 `RasterizationRenderBackendType`（`Gles` / `Indirect`）装配两套渲染器，产出统一的 `Box<dyn SceneRenderer>`；帧的 pass 组装（`use_render_lighting_scene_content`）完全不感知后端差异。Gles 是逐实体 host 绘制（`GLESSceneRenderer`），Indirect 是 id 池 + 间接命令（`IndirectSceneRenderer`）。
- **剔除与光照是 SceneRenderer 的两个消费者**。frustum / 遮挡剔除与阴影贴图都通过「`extract_scene_batch` 取批 + `use_make_scene_batch_pass_content` 画批」复用场景渲染器，批是整条帧链路里流通的通货。

## 核心概念

| 概念 | 定义位置 | 说明 |
| --- | --- | --- |
| `Viewer` | [viewer.rs:3](../../application/viewer-content/src/viewer.rs#L3) | 应用级总装：surfaces_content、selection、viewport 映射、rendering_root、rendering、shared_ctx、字体系统 |
| `RenderingRoot` | [rendering_root.rs:4](../../application/viewer-content/src/rendering_root.rs#L4) | 帧驱动：attachment/pass 池、按 surface 的渲染进程内存、帧统计、`draw_canvas` 入口 |
| `Viewer3dRenderingCtx` | [frame_all.rs:10](../../application/viewer-content/src/rendering/frame_all.rs#L10) | 渲染器常驻维护：culling 配置、后端类型开关、LightSystem、surface→view→视口状态表 |
| `ViewerRendererInstance` | [frame_all.rs:698](../../application/viewer-content/src/rendering/frame_all.rs#L698) | 每帧组装的渲染实例：相机、背景、场景渲染器、批提取器、剔除、裁剪、变换查询 |
| `ViewerBatchExtractor` | [frame_all.rs:713](../../application/viewer-content/src/rendering/frame_all.rs#L713) | 批提取选择器：有增量提取器用增量，否则回退 host 全量提取器 |
| `Viewer3dViewportRenderingCtx` | [frame_viewport.rs:33](../../application/viewer-content/src/rendering/frame_viewport.rs#L33) | 每视口状态：TAA/SSAO/描边/后处理/picker/透明渲染方式/按需渲染缓存 |
| `ViewerCulling` / `ViewerOcclusionCulling` | [rendering/culling.rs:75](../../application/viewer-content/src/rendering/culling.rs#L75)、[:65](../../application/viewer-content/src/rendering/culling.rs#L65) | viewer 侧剔除封装：frustum 执行、OC 状态、剔除结果反馈 |
| `LightSystem` / `SceneLightSystem` | [rendering/lighting/mod.rs:205](../../application/viewer-content/src/rendering/lighting/mod.rs#L205)、[:422](../../application/viewer-content/src/rendering/lighting/mod.rs#L422) | 光照系统配置与每帧光照上下文（含阴影贴图绘制） |
| `LightingRenderingCx` | [lighting/light_pass/mod.rs:13](../../application/viewer-content/src/rendering/lighting/light_pass/mod.rs#L13) | 每帧光照上下文：SceneLightSystem、tonemap、延迟材质注册、光照技术 |
| `QueryGPUHookCx` / `GPUQueryHookStage` | [platform/graphics/webgpu-hook-utils/src/hook.rs:9](../../platform/graphics/webgpu-hook-utils/src/hook.rs#L9)、[:20](../../platform/graphics/webgpu-hook-utils/src/hook.rs#L20) | 渲染侧 hook 上下文：Update / CreateRender 两阶段 |
| `RasterizationRenderBackendType` | [frame_all.rs:5](../../application/viewer-content/src/rendering/frame_all.rs#L5) | 光栅化后端选择：`Gles`（host 逐实体）或 `Indirect`（id 池 + 间接命令） |
| `ViewerDataScheduler` | [data_source.rs:15](../../application/viewer-content/src/data_source.rs#L15) | 纹理/网格 URI 流式调度 + `batch_collector`（帧末实际绘制批次出口） |
| `ViewerSurfaceContent` / `ViewerViewPort` | [lib.rs:119](../../application/viewer-content/src/lib.rs#L119)、[viewport.rs:4](../../application/viewer-content/src/viewport.rs#L4) | 一个 surface 的内容（视口列表 + dpi）；一个视口（矩形、相机、场景） |
| `ViewerFrameRenderingExtension` | [frame_viewport.rs:8](../../application/viewer-content/src/rendering/frame_viewport.rs#L8) | 每视口渲染后的扩展钩子（桌面 viewer 用它画 widget 场景与坐标轴） |
| `ViewerNDC` | [rendering/ndc.rs:4](../../application/viewer-content/src/rendering/ndc.rs#L4) | 自定义 NDC 空间映射（reverse-z 在此注入投影修改） |

## 装配关系与生命周期

### 谁创建谁

`Viewer::new`（[viewer.rs:74](../../application/viewer-content/src/viewer.rs#L74)）是唯一的创建入口，初始化顺序即字段顺序：

```text
Viewer::new(gpu, init_config, worker)
  ├─ Terminal + 默认命令
  ├─ ViewerNDC（reverse-z 开关，来自 init_config.init_only）
  ├─ RenderingRoot::new(&gpu)     // 帧驱动：attachment 池、pass 池、统计、change notifier
  ├─ Viewer3dRenderingCtx::new(gpu, ndc, init_config, font_system)
  │    ├─ culling 配置（OC/frustum 开关、OC 容量上限）→ ViewerCullingConfig
  │    ├─ 后端类型 current_renderer_impl_ty ← init_config.raster_backend_type
  │    ├─ LightSystem::new(&gpu, init_config)   // 光照系统常驻配置
  │    └─ surface_views: 空表（surface 出现时按需建视口状态）
  ├─ SharedHooksCtx（共享计算上下文，帧间常驻）
  └─ font_system（Arc<RwLock<FontSystem>>）
```

`ViewerDataScheduler` 不由 `Viewer` 创建：桌面 viewer（[application/viewer/src/viewer/mod.rs:326](../../application/viewer/src/viewer/mod.rs#L326)）与 C API（[viewer-content-api 的 ViewerAPI::new](../../application/viewer-content-api/src/viewer_api.rs)）各自 `ViewerDataScheduler::new(...)` 后经 `DynCx` 注册——渲染帧与查询调用都必须先注册再使用（见 [viewer-content-api-guide.md](viewer-content-api-guide.md) 的「数据调度与 URI 流式加载」）。`RenderingRoot.draw_canvas` 通过 `dyn_cx` 参数把它接入 `QueryGPUHookCx`，批收集器则从 `scheduler.batch_collector` 每帧取出传给 `render`（[rendering_root.rs:196](../../application/viewer-content/src/rendering_root.rs#L196)）。

### surface 的三处账本

一个 surface 的生命周期状态分散在三个按 `surface_id` 索引的结构里，谁创建、谁销毁必须一一对应：

- `Viewer.surfaces_content: FastHashMap<u32, ViewerSurfaceContent>`——视口列表与 dpi，由宿主维护（桌面 viewer 在 [viewer/mod.rs:384](../../application/viewer/src/viewer/mod.rs#L384) 插入默认视口）。
- `RenderingRoot.render_process_memory: FastHashMap<u32, FunctionMemory>`——每帧 `FrameCtx` 的 hook 内存（attachment/pass 资源的申请使用方记录）按 surface 分账；surface 销毁时经 `drop_surface_render_process_memory`（[rendering_root.rs:55](../../application/viewer-content/src/rendering_root.rs#L55)）清理。
- `Viewer3dRenderingCtx.surface_views: FastHashMap<u32, FastHashMap<u64, Viewer3dViewportRenderingCtx>>`——每视口常驻渲染状态（TAA、SSAO、描边、后处理参数、picker），由 `check_should_render_and_copy_cached`（[frame_all.rs:564](../../application/viewer-content/src/rendering/frame_all.rs#L564)）按当前视口列表增删。

`Viewer.drop_surface`（[viewer.rs:101](../../application/viewer-content/src/viewer.rs#L101)）同时清理后两处。surface 的视口列表变化时，`check_should_render_and_copy_cached` 用「删除不在列表里的视口状态、保留仍在的、新建新的」保持视口状态与视口列表同步（[frame_all.rs:576](../../application/viewer-content/src/rendering/frame_all.rs#L576)）——这正是 `keyed_scope` 依赖的「同 scope 跨帧复用、scope 变化则重建」语义在视口粒度上的体现（见 [skill-translation/frame-pass-assemble-zh.md](skill-translation/frame-pass-assemble-zh.md) 的 scope 约定）。

### 销毁顺序

`drop_viewer_from_dyn_cx`（[viewer.rs:54](../../application/viewer-content/src/viewer.rs#L54)）先 cleanup `Viewer.memory`（用带 SceneWriter 的 `ViewerDropCx`，把内存里的 `EntityHandle` 逐条从数据库删除），**先 drop 掉 dcx 再** `rendering_root.cleanup()`——源码注释警告：渲染根里持有事件源移除器（事件源引用全局 writer），不先销毁会在 drop 时死锁。`Viewer` 的字段按声明顺序 drop，`rendering_root` 先于 `rendering` 被析构，符合「先停帧驱动、再毁渲染器状态」的顺序。

## 一帧的完整走向（用户视角）

### 桌面 viewer 的帧循环

[application/viewer/src/app_loop.rs](../../application/viewer/src/app_loop.rs) 是 winit 事件循环：

```text
WindowEvent::RedrawRequested
  └─ surface.get_current_frame_with_render_target_view(&gpu.device)  // 取交换链帧纹理
       └─ ApplicationCx { draw_target_canvas: canvas, ... }.execute(|cx| app_logic(cx))
            └─ use_viewer(...)（[viewer/mod.rs:311]）
                 ├─ ViewerCx 两轮执行：Gui 阶段（egui 交互、更新输入与相机）→ BaseStage
                 ├─ viewer.update_view_ty_immediate()：收集全部视口相机/尺寸 → viewport_map
                 ├─ viewer.draw_canvas(surface_id, canvas, ...)
                 │    └─ Viewer.draw_canvas → RenderingRoot.draw_canvas（下文）
                 └─ 注销 ViewerDataScheduler
       └─ output.present()  // 交换链呈现
```

`use_viewer` 是桌面应用的每帧壳：egui 交互状态先于渲染帧更新（Gui 阶段），然后 `draw_canvas` 走完整渲染帧。C API 的宿主则直接调 `viewer_render_surface`（见 [viewer-content-api-guide.md](viewer-content-api-guide.md) 的「渲染与读回」），同一份 `draw_canvas` 被两种宿主共用。

### RenderingRoot.draw_canvas：一帧的五个阶段

[rendering_root.rs:77](../../application/viewer-content/src/rendering_root.rs#L77) 是帧的核心驱动器，分五段：

1. **init_frame**（[:62](../../application/viewer-content/src/rendering_root.rs#L62)）：attachment 池与 pass 信息池 tick（跨帧复用计数）、帧号递增、帧耗时统计采样。
2. **FrameCtx 创建**（[:103](../../application/viewer-content/src/rendering_root.rs#L103)）：以本 surface 的 `render_process_memory` 为 hook 内存，`ctx.execute` 包裹整帧。`any_render_change`（`ChangeNotifier`）在此帧被消费——任何系统 `notify_change`（如 egui 参数修改）都会触发重渲染。
3. **按需渲染检查**：`rendering.check_should_render_and_copy_cached`（[frame_all.rs:564](../../application/viewer-content/src/rendering/frame_all.rs#L564)）对每个视口询问「本帧要不要真的渲染」，返回 `requested_render_views: FastHashSet<(viewport_id, idx)>`；`ctx.skip_if_not`（[:122](../../application/viewer-content/src/rendering_root.rs#L122)）在没有任何视口需要渲染时整体跳过——`enable_on_demand_rendering` 下无变化时只拷贝缓存帧，见「按需渲染与帧缓存」一节。
4. **渲染器维护两阶段**（本帧的核心，见下节）：Update 阶段增量维护 → 等任务 → CreateRender 阶段产出 `ViewerRendererInstance` 与 `LightingRenderingCxPrepareCtx`。
5. **render**（[:187](../../application/viewer-content/src/rendering_root.rs#L187)）：`Viewer3dRenderingCtx::render`（[frame_all.rs:607](../../application/viewer-content/src/rendering/frame_all.rs#L607)）逐视口编码全部 pass，然后帧末反馈剔除结果给批收集器。

### 两阶段渲染器维护

`QueryGPUHookCx`（[webgpu-hook-utils/src/hook.rs:9](../../platform/graphics/webgpu-hook-utils/src/hook.rs#L9)）是 query-hook 两阶段模型（[query-hook-guide.md](query-hook-guide.md)）在渲染侧的宿主，阶段用 `GPUQueryHookStage` 表达：

- **Update 阶段**（[rendering_root.rs:133](../../application/viewer-content/src/rendering_root.rs#L133)）：`shared_ctx.flush_drop_queue`（处理共享消费者销毁）→ `QueryGPUHookCx{ stage: Update { spawner, task_pool, immediate_results, inspector } }` → `use_viewer_scene_renderer`（[:147](../../application/viewer-content/src/rendering_root.rs#L147)）。这一遍做**全部增量维护**：纹理系统、批提取器的宿主侧列表与 id 池变更、剔除状态、材质/网格/模型存储的稀疏写入等，计算量大、可并行的部分经 `map_spawn_stage_in_thread` 放进任务池。`waker` 用 `any_render_change` 的 waker——GPU 异步操作完成会唤醒下一帧。
- **等任务**（[:150](../../application/viewer-content/src/rendering_root.rs#L150)）：`pollster::block_on(pool.all_async_task_done())`（wasm 下由外层事件循环驱动，此处为桌面/API 的同步等待），`immediate_results` 并入。
- **CreateRender 阶段**（[:162](../../application/viewer-content/src/rendering_root.rs#L162)）：`QueryGPUHookCx{ stage: CreateRender { task, encoder } }` 再次执行 `use_viewer_scene_renderer` 并 `.unwrap()`。这一遍不做增量计算，只把上一遍维护好的资源**组装成本帧的渲染实例**——`cx.when_render`（`is_in_render()` 为真才执行）就是这一阶段的标记。同时把任务池结果挪到后台任务里 drop（[:183](../../application/viewer-content/src/rendering_root.rs#L183) 的注释：任务池结果 drop 慢）。

注意 `use_viewer_scene_renderer` 是**同一个函数被两阶段各执行一次**：`cx.use_shared_dual_query` 等 hook 调用在 spawn 阶段声明订阅并产出 `UseResult`，resolve 阶段取 `when_render` 的最终值。两阶段的 hook 内存是 `RenderingRoot.render_resource_memory`（跨帧常驻，区别于按 surface 分账、供 `FrameCtx` 用的 `render_process_memory`），所以帧间只重算变化的部分（增量查询的 delta）；`CreateRender` 阶段 `selected_model.register(cx.waker())`（[:175](../../application/viewer-content/src/rendering_root.rs#L175)）让选区变化也能唤醒渲染。

### Viewer3dRenderingCtx::render：逐视口 pass 编码

[frame_all.rs:607](../../application/viewer-content/src/rendering/frame_all.rs#L607) 的 `render`：

- `lighting.prepare(...)`（[:621](../../application/viewer-content/src/rendering/frame_all.rs#L621)）：绘制全部阴影贴图、组装 `LightingRenderingCx`（见「光照系统接入」一节）。
- `culling.set_should_keep_oc_cull_result(batch_collector.will_collecting())`（[:633](../../application/viewer-content/src/rendering/frame_all.rs#L633)）：帧前询问批收集器是否要收集本帧实际绘制批次，决定 OC 是否保留剔除结果。
- 对每个 `(viewport_id, idx)` 进 `requested_render_views`：`ctx.keyed_scope(&viewport_id)`（[:638](../../application/viewer-content/src/rendering/frame_all.rs#L638)）——视口状态（TAA/SSAO/描边等）按视口 id 跨帧复用；设置 `ctx.frame_size = viewport.render_pixel_size()`、相机 uniform、`LODCameraInfo`（视口分辨率 + LOD 阈值）、`active_view_control`，然后 `view_renderer.use_render(...)` 走完整视口 pass 序列（见「逐视口 pass 序列」一节）。LOD 控制与视口激活状态在出 scope 时复位。
- 对不在请求列表里的存活视口 `ctx.skip_keyed_scope(&v.id)`（[:683](../../application/viewer-content/src/rendering/frame_all.rs#L683)）：保持其 hook 状态不重建（帧缓存拷贝路径用）。
- 帧末 `is_collecting()` 为真时 `feedback_culling_result` + `flush_frame`（[:689](../../application/viewer-content/src/rendering/frame_all.rs#L689)）——见「剔除结果的反馈」一节。

## 双后端：WebGPU 间接路径与 GLES 路径的分流

### 分流点

后端选择在两个地方决定：

- `init_config.raster_backend_type`（[init_config.rs:12](../../application/viewer-content/src/init_config.rs#L12)）：`RasterizationRenderBackendType::Indirect`（默认）或 `Gles`。
- `init_config.using_host_driven_indirect_draw`：Indirect 模式下是否退化为 host 驱动（每帧 CPU 遍历实体生成间接命令，用于 GL 后端——WebGL 没有 MIDC 与 GPU 流压缩能力）。

`use_viewer_scene_renderer` 的匹配在 [frame_all.rs:165](../../application/viewer-content/src/rendering/frame_all.rs#L165)，两个分支产出同一形态：`Option<Box<dyn SceneRenderer>>`。`is_indirect = current_renderer_impl_ty == Indirect && !using_host_driven_indirect_draw`（[:147](../../application/viewer-content/src/rendering/frame_all.rs#L147)）进一步决定剔除方式（OC 只在真正的 device 间接路径可用，见「剔除的装配」）。纹理系统按「是否需要间接采样」选型：`get_suitable_texture_system_ty`（[gpu-base/src/texture/mod.rs:239](../../scene/rendering/gpu-base/src/texture/mod.rs#L239)）返回 `GlesSingleBinding`（逐材质绑定，Gles 路径）或 `Bindless` / `TexturePool`（indirect 路径，前者需要 GPU 支持 bindless）。

### Indirect 分支

[frame_all.rs:233](../../application/viewer-content/src/rendering/frame_all.rs#L233)：以「只读存储 buffer」为主体的常驻资源装配——材质存储（unlit/pbr-mr/pbr-sg/occ，可选 storage combine）、节点存储、LOD 属性网格（`use_attribute_lod_mesh_indirect_renderer`）、宽线/宽点/文字渲染器、实例化模型、单元网格，经 `use_indirect_scene_model` 与 `use_viewer_std_model_renderer` 组装，产出 `IndirectSceneRenderer`（[:424](../../application/viewer-content/src/rendering/frame_all.rs#L424)）。批提取器在**非 host-driven** 时于同 scope 内装配增量 key 链（occ layer + 实例化 key + 场景/可见性过滤）并用 `use_occ_incremental_device_scene_batch_extractor` 建 `indirect_extractor`（[:419](../../application/viewer-content/src/rendering/frame_all.rs#L419)）。这条链路的细节（子列表二次分类、provider 创建、host-driven 分支）在 [gpu-indirect-batch-collector-guide.md](gpu-indirect-batch-collector-guide.md) 完整覆盖，这里不重复。

### Gles 分支

[frame_all.rs:166](../../application/viewer-content/src/rendering/frame_all.rs#L166)：逐实体 host 渲染的装配，全部在 `cx.scope` 内：

- 三个特殊渲染器：文字（`use_text3d_gles_renderer`，需要字体系统）、宽线、宽点。
- 网格：`viewer_mesh_input` 的流式网格变化 → `create_sub_buffer_changes_from_mesh_changes` 拆分索引/顶点缓冲 → `use_attribute_mesh_renderer`。
- 材质与模型：四类材质 uniform 合成 `Vec<Box<dyn GLESModelMaterialRenderImpl>>`，模型实现（std model + 宽线 + 宽点 + 文字）合成 `Vec<Box<dyn GLESModelRenderImpl>>`。
- 节点与视依赖变换：`use_node_uniforms` + `use_view_dependent_transform_gles_gpu`（输入是 `SceneModelViewDependentTransformOccShare` 共享查询，见 [viewer-content-api-guide.md](viewer-content-api-guide.md) 的「视依赖变换的增量源」）。
- `use_gles_scene_model_renderer`（[gpu-gles/src/scene_model.rs:3](../../scene/rendering/gpu-gles/src/scene_model.rs#L3)）合成 `GLESPreferredComOrderRenderer`，包成 `GLESSceneRenderer`（[gpu-gles/src/scene.rs:3](../../scene/rendering/gpu-gles/src/scene.rs#L3)）。

各组的 trait 抽象与实现细节（uniform 增量维护、`setup_tex` 直接绑定、`GLESPreferredComOrderRenderer` 的 7 组件组装）见 [gles-material-host-render-guide.md](gles-material-host-render-guide.md)，这里只保留帧组织相关的两点：

- `GLESSceneRenderer` 的 `use_make_scene_batch_pass_content`（[scene.rs:27](../../scene/rendering/gpu-gles/src/scene.rs#L27)）与 indirect 版本行为不同：`batch.get_host_batch()` 取 host 迭代器，`GLESScenePassContent::render` 逐实体 `render_scene_model`（[scene.rs:67](../../scene/rendering/gpu-gles/src/scene.rs#L67)）——每个实体一套状态切换。**所以 Gles 路径的批提取器必须产出 `SceneModelRenderBatch::Host`**：这正是 `use_occ_host_scene_batch_extractor`（[extension/occ-style-draw-control/src/gles.rs:3](../../extension/occ-style-draw-control/src/gles.rs#L3)）的职责——host 全量提取 + occ 分层过滤排序（TopMost 层单独提取，见 [batch-extractor-guide.md](batch-extractor-guide.md) 的分层绘制章节）。

### ViewerBatchExtractor：两路提取器的选择器

[frame_all.rs:713](../../application/viewer-content/src/rendering/frame_all.rs#L713) 的 `ViewerBatchExtractor` 持有两个可选提取器：

```text
ViewerBatchExtractor {
  default_extractor: use_occ_host_scene_batch_extractor（任何后端都有）
  indirect_extractor: Option<...>（仅 Indirect 且非 host-driven 时装配）
}
extract_scene_batch → 有 indirect_extractor 用增量 device 提取，否则回退 host
```

它实现 `SceneBatchBasicExtractAbility`（[gpu-base/src/batch_extraction.rs](../../scene/rendering/gpu-base/src/batch_extraction.rs)），被光照、剔除、widget 场景等所有「取批」方共用。两条提取路径产出不同批形态：增量提取器给 `SceneModelRenderBatch::Device(DeviceSceneModelDrawList)`（id 池 + 子列表），host 提取器给 `SceneModelRenderBatch::Host(Box<dyn HostRenderBatch>)`；`use_make_scene_batch_pass_content` 按批形态分派（host-driven 关闭时 host 批会被现场建池转 device，见 [gpu-indirect-batch-collector-guide.md](gpu-indirect-batch-collector-guide.md) 的「入口与分支」）。

## 剔除的装配

frustum 剔除没有独立 guide，与遮挡剔除的 viewer 侧装配一并在此覆盖（OC 的 GPU 内部机制见 [occlusion-culling-guide.md](occlusion-culling-guide.md)）。

### 装配：use_viewer_culling

[culling.rs:6](../../application/viewer-content/src/rendering/culling.rs#L6) 在 Update 阶段执行，产出 `ViewerCulling`（[:75](../../application/viewer-content/src/rendering/culling.rs#L75)），挂在 `ViewerRendererInstance.culling` 上：

- **OC 状态**（仅 `enable_indirect_occlusion_culling && is_indirect` 时）：按相机持有 `Arc<RwLock<GPUTwoPassOcclusionCulling>>`（[:20](../../application/viewer-content/src/rendering/culling.rs#L20)），键用 `per_camera_per_viewport(viewports, true)` 聚合（多个视口共享同一相机时共用一份 OC 状态，`debug_camera_for_view_related` 会把调试相机与主相机分开）。容量上限来自 `ViewerCullingConfig.occlusion_culling_max_scene_model_count`。
- **包围盒 provider**：`use_scene_model_device_world_bounding` 把 `SceneModelWorldBounding` 共享查询变成 `DrawUnitWorldBoundingProvider`（id → 世界 AABB，OC 与 frustum 的 GPU 侧剔除共用）；同时保留 host 侧查询 `sm_world_bounding`（frustum 的 host 路径与透明排序用）。
- **视锥**：`use_camera_gpu_frustum`（[frustum-culling/src/lib.rs:15](../../scene/rendering/frustum-culling/src/lib.rs#L15)）从 `GlobalCameraTransformShare` 派生 6 平面并上传为 uniform（`CameraGPUFrustums.device`），host 侧另有 `Frustum<f64>` 查询（`CameraGPUFrustums.host`）。
- **开关**：`enable_frustum_culling`。

### frustum 剔除的执行

`use_execute_frustum_culler`（[culling.rs:103](../../application/viewer-content/src/rendering/culling.rs#L103)）按批形态分两条路：

- **device 批**：`cx.access_parallel_compute` 内构造 `GPUFrustumCuller`（frustum uniform + bounding provider + 相机），`batch.use_culled_list_and_do_culling(cx, culler)`（[draw-list/src/stream_compact/mod.rs:11](../../shader/draw-list/src/stream_compact/mod.rs#L11)）——剔除 + 流压缩，产出同构新批（范围 count 压紧、`impl_select_ids` 原样保留）。
- **host 批**：`HostFrustumCulling`（[frustum-culling/src/lib.rs:150](../../scene/rendering/frustum-culling/src/lib.rs#L150)）包装 host 迭代器，CPU 逐实体做 AABB-视锥相交过滤（高精度平移 HPT 减相机世界位置后测试，避免远距离浮点精度问题）。

调用点有两处：不透明批经 `use_draw_with_oc_maybe_enabled`（OC 未启用时，见下）；透明批在 `ViewerTransparentRenderer::use_render` 开头无条件执行（[transparent.rs:81](../../application/viewer-content/src/rendering/transparent.rs#L81)）——透明对象不做遮挡剔除（注释注明 OC 暂不支持透明），frustum 照常。

### 遮挡剔除的执行与结果反馈

`use_draw_with_oc_maybe_enabled`（[culling.rs:141](../../application/viewer-content/src/rendering/culling.rs#L141)）是不透明批的统一入口：

```text
use_draw_with_oc_maybe_enabled(ctx, renderer, scene_pass_dispatcher, camera_gpu, viewport, preflight_content, pass_base, reorderable_batch)
  ├─ OC 启用时跳过 frustum（注释：OC 的遮挡测试同时覆盖了视锥外的对象）
  ├─ 调试分支：viewport.debug_camera_for_view_related 有上帧剔除结果时，
  │    把 drawn_occluder / drawn_not_occluded 两个批经 use_make_scene_batch_pass_content
  │    画进调试视口并直接返回（[:165]）
  ├─ OC 启用：oc_state.write().use_draw(...)（两遍绘制 + 深度金字塔 + 状态回写）
  │    结果按相机存入 oc.culling_results（[:206]）
  └─ OC 未启用：use_make_scene_batch_pass_content 直接画整批
```

帧末的反馈在 `Viewer3dRenderingCtx::render`（[frame_all.rs:689](../../application/viewer-content/src/rendering/frame_all.rs#L689)）：`feedback_culling_result`（[culling.rs:223](../../application/viewer-content/src/rendering/culling.rs#L223)）把每相机的两个结果批依次 `collect_batch` 给 `RenderBatchCollector`，然后 `flush_frame`。是否保留结果由两处协同：帧前 `set_should_keep_oc_cull_result(will_collecting())`（collector 想收集 → OC 本帧保留 `GPUTwoPassOcclusionCullingResult`），或 `always_keep_cull_result`（调试开关）恒保留。collector 的完整语义见 [gpu-indirect-batch-collector-guide.md](gpu-indirect-batch-collector-guide.md) 的「feedback 与 collector」。

## 光照系统接入

### 常驻配置：LightSystem

`LightSystem::new`（[lighting/mod.rs:224](../../application/viewer-content/src/rendering/lighting/mod.rs#L224)）在 `Viewer3dRenderingCtx::new` 时创建一次：光照表面模型（Pbr / SimplePhong）、tonemap、阴影过滤配置（PCF/VSM）、延迟材质注册表（`DeferLightingMaterialRegistry`，注册了 Pbr/Unlit/Phong 三种 encode/decode）、默认前向光照技术。egui 面板运行时改的都是这份配置。

### Update 阶段：use_lighting

[lighting/mod.rs:15](../../application/viewer-content/src/rendering/lighting/mod.rs#L15) 在 `use_viewer_scene_renderer` 末尾调用（所有后端共用）：四类光源 uniform（方向/聚光/点光，配 `MultiLayerTexturePackerConfig` 的 2048³ 阴影图集；区域光；IBL）+ `use_scene_id_provider`，产出 `LightingRenderingCxPrepareCtx`——**常驻增量维护与渲染实例之间的传递物**，与 `ViewerRendererInstance` 一起由两阶段产出。

### render 阶段：LightSystem::prepare 与阴影贴图

[lighting/mod.rs:66](../../application/viewer-content/src/rendering/lighting/mod.rs#L66) 的 `prepare` 是帧内光照的组装点：

- 逐光源更新阴影贴图（`update_shadow_maps`，方向/聚光/点光各一次）：闭包内 `extract_scene_batch(scene, SceneContentKey{ only_alpha_blend_objects: None }, renderer)` 取不透明批（**阴影也走批提取器**，不透明对象才投阴影），`frame_ctx.keyed_scope(&shadow_id)`（阴影序号递增，每个阴影映射一个 scope 缓存），设置 `LODCameraInfo`（阴影相机投影 + 图集分辨率 + LOD 阈值），`renderer.use_make_scene_batch_pass_content(batch)` 画进图集区域（`map_desc.render_ctx`）。
- 组装 `SceneLightSystem`（scene id + 系统 + 光源计算组件组），产出 `LightingRenderingCx`（[light_pass/mod.rs:13](../../application/viewer-content/src/rendering/lighting/light_pass/mod.rs#L13)）：`SceneLightSystem` + tonemap + 延迟材质注册表 + `LightingTechniqueKind`（Forward / DeferLighting）。

`LightingRenderingCx::lighting.get_scene_forward_lighting_component`（[lighting/mod.rs:429](../../application/viewer-content/src/rendering/lighting/mod.rs#L429)）把光源计算、tonemap、emissive 加成、LDR 输出合成一个 `RenderComponent`——这是着色器侧「光照语义」的组装点（`LightableSurfaceTag` / `LDRLightResult` / `DefaultDisplay` 的语义接线，见 [shader-edsl-graphics-zh.md](skill-translation/shader-edsl-graphics-zh.md)）。

### light_pass 的接入位置

`use_render_lighting_scene_content`（[light_pass/mod.rs:20](../../application/viewer-content/src/rendering/lighting/light_pass/mod.rs#L20)）是场景内容真正上屏的函数，被 `Viewer3dViewportRenderingCtx::render_raster` 调用（见下节）。它按光照技术分两个 scope：

- **Forward**（[:82](../../application/viewer-content/src/rendering/lighting/light_pass/mod.rs#L82)）：`pass("scene forward")` 用 `DefaultDisplayWriter::extend_pass_desc` + `g_buffer.extend_pass_desc` 扩展颜色/深度附件，dispatcher 由 blend 禁用器、颜色写入器、g-buffer 写入器、光照组件、clip 组件合成（`RenderArray`）；不透明批进 `use_draw_with_oc_maybe_enabled`（preflight 画背景），透明批由 `ViewerTransparentRenderer::use_render` 处理（NaiveAlphaBlend 与不透明共用同一 pass；Loop32OIT / WeightedOIT 是独立的多 pass 透明管线，[transparent.rs:116](../../application/viewer-content/src/rendering/transparent.rs#L116)）。
- **DeferLighting**（[:143](../../application/viewer-content/src/rendering/lighting/light_pass/mod.rs#L143)）：`pass("scene defer encode")` 只写 g-buffer + 材质缓冲（`FrameGeneralMaterialBufferEncoder`，按注册表写材质索引与参数）；随后 `pass("deferred lighting compute")` 用全屏 quad 从 g-buffer 重建几何（`FrameGeometryBufferReconstructGeometryCtx`）与表面（`FrameGeneralMaterialBufferReconstructSurface`）做延迟光照；透明对象在**另一个 forward pass**（`scene forward transparent in defer mode`，[:222](../../application/viewer-content/src/rendering/lighting/light_pass/mod.rs#L222)）里画。

两种技术都产出一份「画了什么」的剔除结果（OC 的两遍也发生在这里），帧末统一反馈给批收集器。这里不重复批提取与剔除的细节，见 [gpu-indirect-batch-collector-guide.md](gpu-indirect-batch-collector-guide.md) 的「提取与绘制」。

## 逐视口 pass 序列

`Viewer3dViewportRenderingCtx::use_render`（[frame_viewport.rs:342](../../application/viewer-content/src/rendering/frame_viewport.rs#L342)）编码一个视口的完整帧，按序：

1. **渲染目标选择**（[:356](../../application/viewer-content/src/rendering/frame_viewport.rs#L356)）：`should_do_extra_copy`（视口未铺满 surface、或要读回/做帧缓存且目标是 surface 纹理）时申请临时纹理，渲染到临时目标、帧末拷贝回 surface——surface 纹理没有 COPY_SRC usage 时无法读回（[frame_viewport.rs:750](../../application/viewer-content/src/rendering/frame_viewport.rs#L750) 的注释）。
2. **RTX 分支或光栅分支**（[:368](../../application/viewer-content/src/rendering/frame_viewport.rs#L368)）：`rtx_rendering_enabled` 时走 `use_render_ray_tracing`（AO 或参考路径追踪，见 [rendering/ray_tracing.rs](../../application/viewer-content/src/rendering/ray_tracing.rs)），否则 `render_raster`。
3. **光栅分支 render_raster**（[:496](../../application/viewer-content/src/rendering/frame_viewport.rs#L496)）的 pass 序列：

   ```text
   render_raster
     ├─ reproject 更新（相机视投影逆矩阵 + 世界位置，供 TAA/SSAO 重投影）
     ├─ ViewerSceneRenderer 组装（scene 渲染器 + 批提取器 + 相机 + 背景 + 透明渲染器 + 变换查询）
     ├─ 裁剪组件（CSG / 平面数组，use_array_clip 时二选一）
     ├─ content_for_taa 闭包（TAAContent 实现，可按帧抖动相机）：
     │    ├─ scene_result = attachment().sample_count(MSAA?)（HDR 感知）
     │    ├─ g_buffer = FrameGeometryBuffer::new（深度 + 法线 Rgba16Float + 可选 entity_id R32Uint）
     │    ├─ use_render_lighting_scene_content（光照 pass 序列，见上节）
     │    ├─ MSAA resolve（with_color_and_resolve_target）
     │    └─ g_buffer.resolve_if_have_multi_sample（保守深度采样法线解析，g_buffer.rs:77）
     ├─ TAA（enable_taa 时 render_aa_content，否则直接渲染；TAA 帧作为缓存源）
     ├─ FXAA（全屏 pass）
     ├─ PostProcess（post uniform 驱动：色调映射输入、目标 sRGB 感知）
     ├─ highlight compose（选区高亮：Host 批提取选中实体 → 掩码 pass → 高亮合成，RendererArray 带 clip）
     ├─ pass("compose-all")：后处理 + 高亮 + outline（屏幕空间描边，g-buffer 的 entity_id + 法线重投影）
     └─ picker 读回 entity_id 缓冲（GPUxEntityIdMapPicker，enable_gpu_pick_id_write 时）
   ```

   `FrameGeometryBuffer`（[g_buffer.rs:5](../../application/viewer-content/src/rendering/g_buffer.rs#L5)）的 entity_id 通道在 webgl 下不可用（`DownlevelFlags::INDEPENDENT_BLEND` 缺失时降级，[:22](../../application/viewer-content/src/rendering/g_buffer.rs#L22)），拾取/描边依赖的 id 由 picker 的 fallback 路径兜底。

4. **扩展钩子**：`extension.use_draw_content_on_post_frame`（[:393](../../application/viewer-content/src/rendering/frame_viewport.rs#L393)）——桌面 viewer 的实现 `ViewerAppFrameRenderingExtension`（[application/viewer/src/viewer/widget/mod.rs:13](../../application/viewer/src/viewer/widget/mod.rs#L13)）把 widget 场景（坐标轴 + 交互 widget）经「extract + use_make_scene_batch_pass_content + MSAA 渲染 + 预乘 alpha 拷贝」叠到帧上；C API 的 `TopMostStandaloneDraw` 则画 occ 风格 TopMost 图层（见 [viewer-content-api-guide.md](viewer-content-api-guide.md) 的「模板三」）。
5. **收尾**（[:396](../../application/viewer-content/src/rendering/frame_viewport.rs#L396)）：需要时拷贝回 surface 目标；`should_do_frame_caching`（按需渲染开启且非 RTX）时把结果拷贝进缓存帧；`on_encoding_finished` 事件发射 `ViewportRenderedResult`——`read_next_render_result`（[:271](../../application/viewer-content/src/rendering/frame_viewport.rs#L271)）订阅它做异步读回。

### 按需渲染与帧缓存

`enable_on_demand_rendering`（默认开）下，`check_should_render_and_copy_cached`（[frame_viewport.rs:283](../../application/viewer-content/src/rendering/frame_viewport.rs#L283)）维护「上一帧渲染结果副本」：

- 视口尺寸变化、`any_changed`（任何 `notify_change`）、RTX 模式、缓存尺寸不匹配 → 丢弃缓存、本帧重渲染。
- TAA 开启时变更后连续 32 帧不缓存（让 TAA 收敛，[:316](../../application/viewer-content/src/rendering/frame_viewport.rs#L316)）。
- 命中缓存：一个 `CopyFrame` quad pass 直接把缓存拷到目标（[:326](../../application/viewer-content/src/rendering/frame_viewport.rs#L326)），返回 false（不需要重渲染）——帧级上 `RenderingRoot` 由此跳过整个渲染器维护段。

## 常见疑问

- **为什么渲染实例要两阶段产出，不能一次建好？** 增量维护（Update）有真正的计算：提取器宿主列表重排、id 池重定位、存储稀疏写入的收集，这些要派发到线程池、等异步结果；而 `ViewerRendererInstance` 需要拿到最终状态的 GPU 资源句柄（如剔除结果的缓冲、`expect_resolve_stage` 的查询视图）。两阶段把「算」与「组装」分开，组装阶段只做轻量打包。同一函数执行两遍的关键是 hook 状态跨阶段常驻（`render_process_memory`），spawn 阶段写入的增量结果 resolve 阶段直接取。
- **为什么 surface / 视口状态都按 id 分账？** 多 surface 多视口下，资源归属必须能随 surface 销毁独立释放；`keyed_scope` 的缓存语义又要求「scope 身份」稳定——surface id、视口 id、阴影序号、实现 key 都是 scope 身份，任何一处用错都会导致资源抖动或泄漏。
- **为什么 OC 启用时跳过 frustum？** 遮挡测试的 AABB 测试已经排除视锥外对象（深度金字塔对视锥外的不可见对象必然返回被遮挡），重复 frustum 剔除只是浪费一次流压缩（[culling.rs:159](../../application/viewer-content/src/rendering/culling.rs#L159) 的注释）。
- **为什么 Gles 路径用 host 提取器、Indirect 路径用增量提取器？** Gles 逐实体绘制，PSO 切换成本在 pass 内，不需要 id 池；Indirect 需要常驻 id 池 + 子列表按 PSO 分桶，增量维护才划算。`ViewerBatchExtractor` 的 fallback 保证任何后端都有一条可用路径。
- **一帧里 pass 与 scope 什么关系？** pass 是 GPU 命令编码单元；scope 是 hook 资源生命周期单元。同一 scope 的 pass 附件跨帧复用（attachment 池 tick 计数），scope 身份变化（视口增删、实现列表变化）才重建。所以帧骨架的每层（surface → viewport → 阴影 → 实现组）都对应一层 keyed_scope。

## 阅读路线：从窗口事件到像素

- 起点：桌面应用 [app_loop.rs:233](../../application/viewer/src/app_loop.rs#L233) 的 `RedrawRequested` → `use_viewer`（[viewer/mod.rs:311](../../application/viewer/src/viewer/mod.rs#L311)）→ `Viewer.draw_canvas`（[viewer.rs:109](../../application/viewer-content/src/viewer.rs#L109)）。
- 帧驱动：`RenderingRoot.draw_canvas`（[rendering_root.rs:77](../../application/viewer-content/src/rendering_root.rs#L77)）→ 两阶段 `QueryGPUHookCx` → `use_viewer_scene_renderer`（[frame_all.rs:100](../../application/viewer-content/src/rendering/frame_all.rs#L100)，双后端分流在此）。
- 渲染实例：`ViewerRendererInstance`（[frame_all.rs:698](../../application/viewer-content/src/rendering/frame_all.rs#L698)）→ `render`（[frame_all.rs:607](../../application/viewer-content/src/rendering/frame_all.rs#L607)）→ 光照准备 `LightSystem::prepare`（[lighting/mod.rs:66](../../application/viewer-content/src/rendering/lighting/mod.rs#L66)）。
- 视口 pass：`Viewer3dViewportRenderingCtx::use_render`（[frame_viewport.rs:342](../../application/viewer-content/src/rendering/frame_viewport.rs#L342)）→ `render_raster` → `use_render_lighting_scene_content`（[light_pass/mod.rs:20](../../application/viewer-content/src/rendering/lighting/light_pass/mod.rs#L20)）→ 剔除 `use_draw_with_oc_maybe_enabled`（[culling.rs:141](../../application/viewer-content/src/rendering/culling.rs#L141)）→ 批提取与间接命令（[gpu-indirect-batch-collector-guide.md](gpu-indirect-batch-collector-guide.md) 的阅读路线）。
- 上屏：`output.present()`（[app_loop.rs:259](../../application/viewer/src/app_loop.rs#L259)）。

想改「帧里有哪些 pass」改 `render_raster` 与 `use_render_lighting_scene_content`；想换「光栅化后端」改 `raster_backend_type` 配置并对照 [frame_all.rs:165](../../application/viewer-content/src/rendering/frame_all.rs#L165) 的两个分支；想加「每帧后绘制内容」实现 `ViewerFrameRenderingExtension`；想加「每帧实际绘制批次消费者」实现 `RenderBatchCollector` 替换 `ViewerDataScheduler.batch_collector`。

## 延伸阅读

- 间接绘制链路在帧内的组织与批收集器：[gpu-indirect-batch-collector-guide.md](gpu-indirect-batch-collector-guide.md)
- 批提取、增量 id 池与 occ 分层绘制：[batch-extractor-guide.md](batch-extractor-guide.md)
- 两遍遮挡剔除的 GPU 内部机制：[occlusion-culling-guide.md](occlusion-culling-guide.md)
- GPU 绘制列表与流压缩（frustum/OC 的剔除底座）：[draw-list-guide.md](draw-list-guide.md)
- 间接材质参数与纹理系统（Indirect 分支的材质侧）：[material-indirect-render-guide.md](material-indirect-render-guide.md)
- 嵌入层 C API 的宿主视角：[viewer-content-api-guide.md](viewer-content-api-guide.md)
- 渲染帧组装与 FrameCtx：[skill-translation/frame-pass-assemble-zh.md](skill-translation/frame-pass-assemble-zh.md)
- 两阶段 hook 执行与共享计算：[query-hook-guide.md](query-hook-guide.md) / [hooks-guide.md](hooks-guide.md)
- 渲染侧 GPU hook 基建（QueryGPUHookCx 的 Update/CreateRender 两阶段、SparseBufferWritesSource、GrowableRangeAllocator）：[webgpu-hook-utils-guide.md](webgpu-hook-utils-guide.md)
- GLES 材质与模型 host 渲染路径（GLESModelMaterialRenderImpl、setup_tex 直接绑定）：[gles-material-host-render-guide.md](gles-material-host-render-guide.md)
