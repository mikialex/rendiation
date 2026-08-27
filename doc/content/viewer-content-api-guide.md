# Rendiation Viewer-Content-API 指南（application/viewer-content-api）

本文梳理 [application/viewer-content-api](../../application/viewer-content-api) 的嵌入层实现：它以 `cdylib` + cbindgen 的形式把整个 rendiation viewer 封装成 C ABI（`bindings.h`），供外部宿主（原生窗口应用、其他语言绑定）调用。文档聚焦有实质内容的实现：多 surface 管理、独立的查询系统（拾取查询与世界派生查询），以及把两者衔接起来的 `ViewerAPICx` 两阶段执行壳。

## 前置阅读

viewer-content-api 不包含渲染器与场景模型本身，它是这些子系统的"宿主壳"。阅读前建议先了解：

| 文档 | 内容 |
| --- | --- |
| [skill-translation/database-schema-zh.md](skill-translation/database-schema-zh.md) | 类型安全关系数据库：实体/组件/外键、全局数据库与读写视图 |
| [skill-translation/scene-core-structure-zh.md](skill-translation/scene-core-structure-zh.md) | 场景实体类型（SceneEntity、SceneNodeEntity、SceneCameraEntity、SceneModelEntity 等）与组件 |
| [skill-translation/query-system-zh.md](skill-translation/query-system-zh.md) | 增量查询（DualQuery / DataChanges / ValueChange） |
| [hooks-guide.md](hooks-guide.md) | hook 运行时：FunctionMemory、scope、动态/静态阶段 |
| [query-hook-guide.md](query-hook-guide.md) | 两阶段执行模型（spawn/resolve）、共享计算、任务池与 waker 传播 |
| [geometry-query-guide.md](geometry-query-guide.md) | 场景拾取与几何查询抽象（ViewerPicker、SceneModelPicker、IterProvider） |
| [skill-translation/frame-pass-assemble-zh.md](skill-translation/frame-pass-assemble-zh.md) | 多通道渲染帧组装（FrameCtx、pass、attachment） |
| [skill-translation/viewer-scene-building-zh.md](skill-translation/viewer-scene-building-zh.md) | viewer 场景构建实践（网格、材质、灯光） |
| [skill-translation/rendiation-algebra-zh.md](skill-translation/rendiation-algebra-zh.md) | 数学库（Vec/Mat/Box3） |

## 模式概览

viewer-content-api 的定位是"把 Rust 渲染引擎接进非 Rust 宿主"。它自身很薄——核心逻辑全部在依赖 [application/viewer-content](../../application/viewer-content)（Viewer 本体）与更底层的 database / query-hook / scene 系列里，本 crate 负责：

- 把**数据库写入**翻译成 C 函数（`create_node`、`create_mesh`、`scene_model_set_mesh`……直接写全局数据库组件）。
- 管理**多个渲染目标**（窗口 surface 与离屏纹理），驱动渲染帧，提供结果读回。
- 提供两套**独立查询 API**：surface 级的拾取查询与全局的世界派生查询，都不依赖 GPU 渲染结果，直接查数据库派生数据。

整体分层：

```text
外部宿主（C/C++ 或其他语言，经 bindings.h）
  └─ viewer-content-api（cdylib，本 crate）
       ├─ ViewerAPI：surface 管理 + 帧驱动 + 查询工厂
       │    ├─ Viewer（viewer-content）：surfaces_content / rendering_root / rendering / shared_ctx / font_system
       │    ├─ ViewerDataScheduler（viewer-content）：纹理/网格 URI 流式加载与批收集
       │    └─ ViewerAPICx：两阶段执行壳（spawn 收集任务 → resolve 取回结果）
       ├─ ViewerQueryAPI（surface 级拾取：射线 / 范围 / 子 primitive）
       └─ ViewerWorldDeriveQueryAPI（全局派生：节点世界矩阵、模型世界/局部包围盒、场景包围盒）
  底层子系统：database（全局数据库）→ query-hook（增量计算与共享）→ scene/core（场景模型）
              → scene/geometry-query（拾取）→ scene/rendering（GPU 渲染）
```

主数据流：

```text
rendiation_init（建全局数据库、注册数据模型、可选开启追踪）
  → create_viewer_content_api_instance（读 toml 配置，初始化 GPU 与 Viewer）
  → viewer_create_surface（建 GPU surface + 默认场景/相机/节点实体 + viewport）
  → C 实体接口写全局数据库（节点/网格/材质/模型/灯光…）
  → viewer_render_surface（取当前帧目标 → RenderingRoot 帧流水线 → present）
  → viewer_read_last_render_result（离屏 surface 读回，包装成纹理实体）
  → viewer_create_picker_api / viewer_create_world_derive_query_api（查询）
```

## 核心概念

| 概念 | 定义 | 说明 |
| --- | --- | --- |
| `ViewerAPI` | [application/viewer-content-api/src/viewer_api.rs](../../application/viewer-content-api/src/viewer_api.rs) | API 根对象：surface 创建/销毁/调整、帧驱动、查询工厂；内部持 `ViewerAPICore` 与两个持久 `FunctionMemory` |
| `ViewerAPICore` | 同上 | 实际状态：GPU、surface 表（`FastHashMap<u32, ViewerCanvas>`）、`Viewer`、任务生成器、数据调度器、`DynCx`、异步任务池与即时结果 |
| `ViewerCanvas` | 同上 | surface 的两种形态：`Surface(SurfaceWrapper)`（窗口）与 `Offscreen(RenderTargetView)`（离屏纹理） |
| `ViewerSurfaceContent` | [application/viewer-content/src/lib.rs](../../application/viewer-content/src/lib.rs) | 每个 surface 的场景内容：`Vec<ViewerViewPort>` + device pixel ratio |
| `ViewerViewPort` | [application/viewer-content/src/viewport.rs](../../application/viewer-content/src/viewport.rs) | 一个视口：相对 surface 的物理像素矩形、相机/相机节点/场景实体句柄 |
| `ViewerAPICx` | [application/viewer-content-api/src/cx.rs](../../application/viewer-content-api/src/cx.rs) | API 层的 hook 上下文，实现 `HooksCxLike` + `QueryHookCxLike` + `DBHookCxLike`，把一次 API 调用拆成 spawn/resolve 两阶段 |
| `ViewerQueryAPI` | [application/viewer-content-api/src/viewer_api.rs](../../application/viewer-content-api/src/viewer_api.rs) | surface 级拾取查询对象：`pick_list` / `pick_range` / `pick_range_sub_primitive` / `get_camera_position_world` |
| `ViewerWorldDeriveQueryAPI` | 同上 | 全局派生查询对象：节点世界矩阵、模型世界/局部包围盒、`SceneBoundingComputer` |
| `SceneBoundingComputer` | [application/viewer-content-api/src/bbox.rs](../../application/viewer-content-api/src/bbox.rs) | 场景包围盒计算：两个动态 BVH（是否含 view 依赖/无限模型）+ 按 view 的扩展 |
| `SceneModelLocalBounding` | [application/viewer-content/src/bounding.rs](../../application/viewer-content/src/bounding.rs) | 共享派生：各几何类型（网格/宽线/宽点/文字/单元网格/实例化）局部包围盒的并集 |
| `SceneModelWorldBounding` | 同上 | 共享派生：世界矩阵 × 局部包围盒；含视依赖变换的模型返回 `None`（动态，不进 BVH） |
| `SceneModelViewDependentTransformOccShare` | [application/viewer-content/src/view_dependent_transform.rs](../../application/viewer-content/src/view_dependent_transform.rs) | 共享派生：视依赖变换（按 view 变化的模型矩阵），`use_compute_incremental_source_by_diffing` 由全量视口表差分出增量源 |
| `ViewerDataScheduler` | [application/viewer-content/src/data_source.rs](../../application/viewer-content/src/data_source.rs) | 纹理/网格的 URI 流式加载调度器与渲染批收集器；经 `DynCx` 按类型注册 |
| `APITraceEventSender` | [application/viewer-content-api/src/trace.rs](../../application/viewer-content-api/src/trace.rs) | API 事件追踪发送器，配合数据库变更追踪写入 trace 文件 |
| `ViewerEntityHandle` | [application/viewer-content-api/src/lib.rs](../../application/viewer-content-api/src/lib.rs) | C 边界句柄 `{index, generation}`，与 `EntityHandle<T>` / `RawEntityHandle` 互转 |
| `TopMostStandaloneDraw` | [application/viewer-content-api/src/top_most_standalone_draw.rs](../../application/viewer-content-api/src/top_most_standalone_draw.rs) | 渲染扩展：帧末尾绘制 occ-style 的 top-most 层（MSAA resolve + 拷贝到目标） |
| `ViewerFrameRenderingExtension` | [application/viewer-content/src/rendering/frame_viewport.rs](../../application/viewer-content/src/rendering/frame_viewport.rs) | 每帧渲染后钩子 trait，`TopMostStandaloneDraw` 是其实现 |

## 初始化与全局环境

### rendiation_init：一次性的全局设置

C 侧所有接口的先决条件（[c_api/init.rs](../../application/viewer-content-api/src/c_api/init.rs)）：

- 安装 panic 钩子：打印 payload 与 backtrace，追加写 `rendiation_panic.txt`，然后 `abort`——C 边界不跨语言 unwind。
- 初始化 `env_logger`。
- `setup_global_database` + `enable_label_for_all_entity`：创建全局数据库（见 [skill-translation/database-schema-zh.md](skill-translation/database-schema-zh.md)）。
- `register_viewer_content_data_model()`（[viewer-content/src/lib.rs](../../application/viewer-content/src/lib.rs)）：注册场景、选择、阴影、裁剪、区域光、宽点、文字、occ 风格材质、实例化模型、单元网格等全部实体与组件——**这是数据模型可见性的来源**，之后 C 接口才能写这些组件。
- 声明 `SceneModelIsInfinity` 组件（场景包围盒计算专用标记，见 [bbox.rs](../../application/viewer-content-api/src/bbox.rs) 的消费）。
- `setup_tracing(trace_write_path)`：见"事件追踪"一节。

### ViewerAPI::new：初始化 GPU 与 Viewer

`create_viewer_content_api_instance(config_path)`（[c_api/viewer.rs](../../application/viewer-content-api/src/c_api/viewer.rs)）把 toml 路径读成 `ViewerInitConfig`（[viewer-content/src/init_config.rs](../../application/viewer-content/src/init_config.rs)，`#[serde(default)]`，解析失败退回默认值），随后 `ViewerAPI::new`（[viewer_api.rs](../../application/viewer-content-api/src/viewer_api.rs)）：

- `init_config.make_gpu_platform_config()` → `GPU::new`（wgpu 后端、shader 运行时保护、调试开关）。
- `TaskSpawner::new("viewer-api", thread_count)`：rayon 线程池，供两阶段执行的异步计算使用。
- `Viewer::new(gpu, &init_config, worker)`（[viewer-content/src/viewer.rs](../../application/viewer-content/src/viewer.rs)）：创建 `RenderingRoot`（帧流水线）、`Viewer3dRenderingCtx`（渲染器维护）、`Terminal`、`SharedHooksCtx`、字体系统等。
- `ViewerDataScheduler::new(None)`：纹理/网格流式调度器（内存后端）。
- 组装 `ViewerAPICore`，`ViewerAPI` 持有两个独立的 `FunctionMemory`（`picker_mem`、`world_derive_access_mem`），供查询对象跨调用持久状态。

`ViewerInitConfig` 里有几组值得注意的开关：`raster_backend_type`（Gles / Indirect）、`init_only`（不可运行时变更：线程数、reverse-z、wgpu 后端覆盖、shader 保护、dxc 路径）、渲染特性（TAA/FXAA/MSAA、阴影、网格地面、按需渲染 `enable_on_demand_rendering`）、拾取特性（`use_scene_bvh`、`use_array_clip`）。

## 多 surface 管理

`ViewerAPICore` 用一张 `FastHashMap<u32, ViewerCanvas>` 管理多个渲染目标（[viewer_api.rs](../../application/viewer-content-api/src/viewer_api.rs)），同时 `viewer.surfaces_content` 里每个 surface id 对应一份 `ViewerSurfaceContent`（视口 + dpi）。surface id 从 0 递增分配。

### 创建 surface

`create_surface(init, width, height)` 分三步：

- 按 `ViewerAPIInit`（[viewer-content-api-trace-info/src/lib.rs](../../application/viewer-content-api-trace-info/src/lib.rs)）构建 `ViewerCanvas`：`Surface` 分支用 win32 窗口句柄（`hwnd`/`hinstance`）经 `raw_gpu` 创建 wgpu surface 并包进 `SurfaceWrapper`（[gpu_with_surface.rs](../../application/viewer-content/src/gpu_with_surface.rs)，内部 `Arc<RwLock<GPUSurface>>` + 持有窗口句柄的 `_surface_holder` 防止句柄先于 surface 销毁）；`Offscreen` 分支用 `PooledTextureKey` 建一张 `Rgba8UnormSrgb` 纹理（RENDER_ATTACHMENT + COPY_SRC），包成 `RenderTargetView`。
- 为每个 surface 建**默认场景**：`SceneEntity`、一个 `Mat4::lookat` 相机节点、`SceneCameraPerspective` 相机、写灰色 `SceneSolidBackground`，组装一个覆盖全 surface 的 `ViewerViewPort` 塞进 `surfaces_content`。
- 发射 `CreateSurface` 追踪事件并调用 `resize`。

创建时源码注释提示：默认节点/场景/相机实体在 surface 销毁时不会自动回收（todo 项），目前视为可接受泄漏。

### 生命周期与变更

| 方法 | 行为 |
| --- | --- |
| `drop_surface` | 移除 `surfaces` 与 `surfaces_content`，并让 `Viewer.drop_surface` 清理该 surface 的渲染进程内存（attachment/缓存）与视图状态 |
| `resize` | 同时更新 `ViewerCanvas` 尺寸（离屏则重建纹理）与 viewport 的宽高字段 |
| `set_device_pixel_ratio` | 更新 dpi，供逻辑像素换算（拾取输入的物理换算依据） |
| `set_surface_scene` | 替换 viewport[0] 的 `SceneEntity` |
| `set_surface_camera` | 替换相机并反查 `SceneCameraNode` 同步更新相机节点 |

### 渲染与读回

`render_surface(surface_id)` 是纯同步调用：

- `ViewerCanvas::Surface`：`get_current_frame_with_render_target_view` 取帧纹理（处理 Occluded/Timeout/子优），拿到后 `present()`。
- `ViewerCanvas::Offscreen`：直接克隆 `RenderTargetView`。
- 注册 `ViewerDataScheduler` 到 `DynCx`，调用 `viewer.draw_canvas(surface_id, target, ...)` 走完整帧流水线（见下节），传入的渲染扩展是 `TopMostStandaloneDraw`（携带该 surface 的场景与 reverse-z 配置）。

`read_last_render_result(surface_id)` 只对离屏 surface 生效：把目标纹理经 encoder `read_texture_2d` 读回 `GPUBufferImage`（原始未对齐像素数据），再包装成一个 `SceneTexture2dEntity` 实体（`MaybeUriData::Living` + `ExternalRefPtr`）返回其句柄——读回结果也以实体形式进入数据库，可被材质引用。

每帧渲染状态按 surface 隔离：`RenderingRoot.render_process_memory`（`FastHashMap<u32, FunctionMemory>`，渲染进程的 hook 状态——attachment 与 pass 的申请使用方——按 surface 分账，见 [rendering_root.rs](../../application/viewer-content/src/rendering_root.rs)），`Viewer3dRenderingCtx.surface_views`（surface id → view id → `Viewer3dViewportRenderingCtx`，每视口持有 TAA/SSAO/描边/后处理等状态）。`enable_on_demand_rendering` 时 `check_should_render_and_copy_cached` 会在内容无变化时把缓存帧直接拷到目标，避免无谓重渲染。

## 帧驱动与 ViewerAPICx 两阶段执行

### viewer_api_cx_scope：一次 API 调用 = 一帧的派生计算

查询 API 的创建（`create_query_api` / `create_world_derive_query_api`）都在 `viewer_api_cx_scope`（[viewer_api.rs](../../application/viewer-content-api/src/viewer_api.rs)）里执行——这是 query-hook 两阶段模型（[query-hook-guide.md](query-hook-guide.md)）在 API 层的驱动器：

```text
注册 ViewerDataScheduler 到 DynCx
viewer.shared_ctx.flush_drop_queue：处理共享计算的消费者销毁
viewer.shared_ctx.reset_visiting：清空跨轮共享状态
Spawn 阶段：ViewerAPICxStage::Spawn（spawner + AsyncTaskPool + immediate_results）
  —— 声明订阅、同步计算、把异步任务装进池
pollster::block_on(pool.all_async_task_done)：等待全部任务
  —— 结果进 TaskPoolResultCx，immediate_results 并入
Resolve 阶段：ViewerAPICxStage::Resolve（TaskPoolResultCx）
  —— 按 token 取回结果，组装最终 API 对象
注销 ViewerDataScheduler
```

闭包 `f` 在两个阶段各执行一次：spawn 阶段返回 `None`（`assert!(r.is_none())`），resolve 阶段返回最终结果（`when_resolve_stage` 内组装）。`FunctionMemory` 由调用者传入——`picker_mem` / `world_derive_access_mem` 是 `ViewerAPI` 的持久字段，所以两次调用之间 hook 状态（如共享 consumer token、派生缓存）得以保留，而每次调用开头都要 `setup_new_frame_allocator` 重建帧分配器。

### ViewerAPICx：hook 运行时集成

[application/viewer-content-api/src/cx.rs](../../application/viewer-content-api/src/cx.rs) 的 `ViewerAPICx<'a>` 是 query-hook 的 `QueryHookCxLike` 实现（与桌面 viewer 的 `ViewerCx`、渲染的 `QueryGPUHookCx` 并列的第三个宿主）：

- `stage()` 把本 crate 的 `ViewerAPICxStage` 映射为 `QueryHookStage::SpawnTask` / `ResolveTask`，`is_spawning_stage` / `is_resolve_stage` 相应判断。
- `waker` 用 noop waker（同步 API，无外部事件循环）。
- `use_shared_consumer` 经 `use_state_init` 创建 `SharedConsumerToken`（消费身份登记进 `Viewer.shared_ctx`），`shared_hook_ctx` 直通 viewer。
- `use_state_init` 注册状态时带 `ViewerAPICxDropCx` 清理回调；`CanCleanUpFrom` 为 `SharedConsumerToken` 与 `NothingToDrop<T>` 提供空实现。
- `InspectableCx::if_inspect` 为空操作（API 场景不需要 inspector）。
- `flush` 仅在 spawn 阶段执行（与渲染侧惯例一致），drop 用 `ViewerAPICxDropCx`（只含 `DynCx`）。

`ViewerAPI::drop` 同样走清理路径：先发射 `DropViewer` 追踪事件，随后依次 cleanup 两个 FunctionMemory，再 `drop_viewer_from_dyn_cx`（[viewer-content/src/viewer.rs](../../application/viewer-content/src/viewer.rs)）——它 cleanup `Viewer.memory` 并先 `drop(dcx)` 再 `rendering_root.cleanup()`（注释警告：渲染根含事件源移除器，必须先行销毁，否则持有 writer 死锁）。

### 渲染帧流水线（Viewer.draw_canvas → RenderingRoot）

渲染本身在 viewer-content：`RenderingRoot.draw_canvas`（[rendering_root.rs](../../application/viewer-content/src/rendering_root.rs)）：

- `init_frame`：attachment 池与 pass 信息池 tick、帧序号递增、帧耗时统计。
- 以 surface 对应的 `render_process_memory` 建 `FrameCtx`。
- `check_should_render_and_copy_cached`：按需渲染时无变化则直接拷贝缓存帧。
- **维护渲染器**两阶段（`QueryGPUHookCx`，[platform/graphics/webgpu-hook-utils](../../platform/graphics/webgpu-hook-utils)）：`GPUQueryHookStage::Update` 阶段执行 `use_viewer_scene_renderer`（[frame_all.rs](../../application/viewer-content/src/rendering/frame_all.rs)）——纹理系统、剔除、材质/网格/模型 GPU 资源（Gles 或 Indirect 两条后端路径）、occ 批提取器等全部用 hook 维护；`block_on` 等待任务后进入 `CreateRender` 阶段，产出 `ViewerRendererInstance`（相机渲染器、背景、场景渲染器、批提取器、剔除、相机变换/世界包围盒查询视图）。
- `Viewer3dRenderingCtx::render`：按 `requested_render_views` 逐视口 `keyed_scope` 组装 pass（光照、透明、描边、TAA、后处理、网格地面），最后调用 `ViewerFrameRenderingExtension::use_draw_content_on_post_frame`。

`TopMostStandaloneDraw`（[top_most_standalone_draw.rs](../../application/viewer-content-api/src/top_most_standalone_draw.rs)）即这个扩展钩子的使用：从 `ViewerBatchExtractor` 的 occ 批提取器（Indirect 用 `OccStyleOrderControlSceneBatchExtractor`，Gles 用其 Gles 变体）取 `get_top_most_layer(scene)`（提取侧机制见 [batch-extractor-guide.md](batch-extractor-guide.md)），配前向光照组件与相机 UBO，先渲到 4x MSAA 附件（reverse-z 时深度清 0），resolve 后经 `copy_frame`（预乘 alpha 混合）拷进最终目标——把 occ 风格"永远置顶"的图层叠到帧之上。

## 独立的查询系统

查询 API 与渲染管线完全解耦：不读任何 GPU 结果，全部来自数据库上的共享派生查询（[query-hook-guide.md](query-hook-guide.md) 的 `use_shared_dual_query` 体系）。两类查询对象：

### ViewerQueryAPI：surface 级拾取

`create_query_api(surface_id)` 在 spawn 阶段调用 `use_viewer_scene_model_picker_impl`（[pick.rs](../../application/viewer-content/src/pick.rs)，与桌面 viewer 共用同一组装），resolve 阶段把 picker 的 `active_view` 设为该 surface 首个 viewport 的 id（视依赖变换按 view 查矩阵）。

| 方法 | 语义 |
| --- | --- |
| `pick_list(x, y, tolerance, remove_clipped)` | 逻辑像素坐标反投影成射线（`create_viewport_pointer_ctx` 找最上层 viewport），`pick_models_all` 取全部命中；`remove_clipped` 时用 `ArrayClipPickFilter` 过滤被裁剪面切掉的命中；输出 `ViewerRayPickResult { primitive_index, hit_position, scene_model_handle }` |
| `pick_range(ax, ay, bx, by, contain, precise, tolerance)` | 屏幕矩形 → `create_range_pick_frustum` 世界视锥 → `range_pick_models` 按 `ObjectTestPolicy::Contains/Intersect` 收集命中模型 |
| `pick_range_sub_primitive(...)` | 对调用者指定的模型列表逐模型做 `frustum_query_sub_primitives`，输出 `(model, primitive_index)` 对——模型级范围拾取细化到 primitive |
| `get_camera_position_world` | 从 `camera_transforms` 共享查询读该 surface 相机世界位置 |

拾取的几何求交细节见 [geometry-query-guide.md](geometry-query-guide.md)，这里只强调 API 层两点：所有像素输入都是**逻辑像素**（内部除以 device pixel ratio 换算物理像素）；命中排序（near-to-far）与结果装箱发生在 C 边界层（`picker_pick_list` 里按相机距离排序后 `Box::leak`）。

### ViewerWorldDeriveQueryAPI：全局派生查询

`create_world_derive_query_api()` 与 surface 无关，只暴露只读查询视图（[viewer_api.rs](../../application/viewer-content-api/src/viewer_api.rs)）：

```rust
pub struct ViewerWorldDeriveQueryAPI {
  pub world_mats: BoxedDynQuery<RawEntityHandle, Mat4<f64>>,
  pub sm_world_bound: BoxedDynQuery<RawEntityHandle, Option<Box3<f64>>>,
  pub sm_local_bound: BoxedDynQuery<RawEntityHandle, Box3<f32>>,
  pub scene_bounding: SceneBoundingComputer,
}
```

组装自三个共享派生：`use_global_node_world_mat_view`（[scene/core/src/node.rs](../../scene/core/src/node.rs)，节点世界矩阵的树形归约）、`SceneModelLocalBounding(font_system)` 与 `SceneModelWorldBounding(font_system)`（见下）。注意这里用的是 `use_shared_dual_query_view`——只共享 view、不消费 delta（`skip_change`），因为查询只读、且与渲染侧（如 `ViewerRendererInstance.sm_world_bounding`）共享同一份 upstream。

### SceneBoundingComputer：场景包围盒

[application/viewer-content-api/src/bbox.rs](../../application/viewer-content-api/src/bbox.rs) 是"独立查询系统"里最完整的实现。它维护两个按场景划分的动态 BVH（[extension/dynamic-bvh-scene](../../extension/dynamic-bvh-scene)）：

```text
SceneModelWorldBounding → filter_map(Some) → union(SceneModelVisible == true)
  └─ fork
     ├─ union(SceneModelIsInfinity == false) → BVH：visible_no_view_dep_no_infinity_bvh
     └─ BVH：visible_no_view_dep_bvh
```

`get_or_compute_scene_bounding(scene, consider_view_dep, consider_infinity)`：

- 取对应 BVH 的根 AABB（`get_root_aabb`）。
- `consider_view_dep = Some(view_id)` 时，遍历 `view_maps`（`SceneModelViewDependentTransformOccShare`，键 `ViewSceneModelKey`）中属于该 view 的模型，逐一检查可见性与 infinity 开关，用 `sm_to_local_bbox` 的局部包围盒乘视依赖矩阵展开进结果——注释说明这类"view 依赖对象"不多，全量遍历可接受；而 `consider_view_dep` 为 None 时 BVH 之外的视依赖对象被忽略。

BVH 的 margin 在这里为常数 0（与拾取侧的宽线/宽点 margin 不同），`use_bvh` 的 `dual_query_map(|_| 0.)` 即此意。

### 包围盒的共享派生链

`SceneModelLocalBounding`（[bounding.rs](../../application/viewer-content/src/bounding.rs)）把各几何类型的局部包围盒合并成一份：属性网格标准模型（`SceneModelByAttributesMeshStdModelLocalBounding`，输入经 `viewer_mesh_input`）、宽线、宽点、文字（`Text3dSceneModelLocalBounding(font_system)`）、单元网格，经 `dual_query_select` 链式取首个命中，再与实例化模型包围盒（`use_instanced_model_local_bounding`）并集。

`SceneModelWorldBounding` 则是"世界矩阵 × 局部包围盒"的交集（`dual_query_intersect`），并减掉含 `SceneModelViewDependentTransformOcc` 的模型（返回 `None`——它的矩阵随视角变化，不进静态 BVH），最后物化为哈希表（`use_dual_query_materialized_hashmap`）。

### 视依赖变换的增量源

`SceneModelViewDependentTransformOccShare(ndc, viewports_map)`（[view_dependent_transform.rs](../../application/viewer-content/src/view_dependent_transform.rs)）的输入 `viewports_map`（view id → (相机, 视口尺寸)）是**全量快照**。`use_compute_incremental_source_by_diffing` 把它转成增量查询：spawn 阶段与 `use_shared_hash_map` 中的旧快照做差（删除消失项、更新变化项），产出 `DualQuery { view, delta }`——注释强调"map 应保持小"以维持差分的成本。下游再经 `use_occ_style_view_dependent_transform_data` 合成视依赖矩阵。

拾取侧的 `ViewerQbvhShared`（[pick.rs](../../application/viewer-content/src/pick.rs)）同理：`SceneModelWorldBounding` fork 后并上宽线（线宽）与宽点（逐顶点最大宽度）的 margin 查询，喂给 `use_scene_dynamic_bvh`——拾取 BVH 的 margin 是非零的，保证屏幕空间容差下宽线/宽点能被命中。

## 数据调度与 URI 流式加载

`ViewerDataScheduler`（[data_source.rs](../../application/viewer-content/src/data_source.rs)）提供两类异步资源流：

- 纹理：`viewer_texture_input` 订阅 `SceneTexture2dEntityDirectContent` 的变化，产出 `DataChangesAndLivingReInit<u32, Arc<GPUBufferImage>, Arc<String>>`——`Uri` 形式的直接内容经 `TextureScheduler`（`NoControlStreaming`）按 URI 异步加载；后端 `texture_uri_backend` 在非 wasm 且给了目录时是 `URIDiskSyncSource`（磁盘缓存 + rmp-serde 序列化），否则是内存源。
- 网格：`viewer_mesh_input` 把属性网格实体与其顶点/索引缓冲关系的变化（`AttributesMeshEntity` 集合、`SceneBufferViewBufferId/Range`、外键反查）归并成"需要重读的网格集合"，`load_uri_mesh` 并发加载各 `AttributeUriData` 缓冲并组装 `AttributesMeshWithVertexRelationInfo`。

两类输入都是 `DBHookCxLike` 下的共享 provider（`DBMeshInput` / `DBTextureUriInput`），经 `access_cx!` 从 `DynCx` 取调度器——这就是 `viewer_api_cx_scope` 与渲染帧开头都要注册/注销 `ViewerDataScheduler` 的原因。`batch_collector` 默认是 `DoNothingRenderBatchCollector`（渲染侧需要时替换）。

## 事件追踪

`rendiation_init` 的 `trace_write_path` 非空时（[trace.rs](../../application/viewer-content-api/src/trace.rs)）：

- 建 `FileTraceWriter<TracingMessage<RendiationCxAPITraceEvent>>`，调用 `start_tracing(&global_database(), writer)`（[utility/database-tracing/src/lib.rs](../../utility/database-tracing/src/lib.rs)）——为所有已注册实体/组件挂数据监听，把**每次数据库变更**（实体创建/删除、字段写入）序列化进文件，并在头部写名称表。
- 同时 `APITraceEventSender` 在 API 的每个关键入口发射 `RendiationCxAPITraceEvent`（[viewer-content-api-trace-info/src/lib.rs](../../application/viewer-content-api-trace-info/src/lib.rs)）：`CreateSurface`/`DeleteSurface`/`ResizeSurface`/`Render`/`CreatePicker`/`PickerPickList`/`PickRange`/`PickSubPrimitiveRanges`/`SceneBoundingQuery`/`DropViewer` 等，`ViewerAPIInit`（Offscreen / Surface{hwnd,hinstance}）也被序列化进事件。该类型实现 `TraceReplayTarget`（判别值 11），标记"可重放"。
- `expect_tracing_event_emitter` 从 `OnceLock` 取发送器，`rendiation_init` 之前调用任何 API 会 panic。

这条链路与桌面 viewer 的追踪（[application/viewer/src/db_tracing.rs](../../application/viewer/src/db_tracing.rs)）共用 database-tracing：同一套二进制格式，可用 `trace_to_text` 转成文本排查交互序列。

## 句柄与 C 边界

`ViewerEntityHandle`（[lib.rs](../../application/viewer-content-api/src/lib.rs)）是 `#[repr(C)]` 的 `{ index: u32, generation: u64 }`，提供空句柄（`u32::MAX`/`u64::MAX`）与 `EntityHandle<T>`（带类型）、`RawEntityHandle`（无类型）三个方向的转换。C 函数接受句柄后 `into()` 成类型化句柄直接操作全局数据库。

`build.rs` + `cbindgen.toml` 在构建时生成 `bindings.h`（含全部 `#[unsafe(no_mangle)] extern "C"` 函数与结构体布局）。函数按域分组：初始化与实例生命周期（`rendiation_init`、`create_viewer_content_api_instance`）、surface 操作（`viewer_create_surface`、`viewer_create_offscreen_surface`、`viewer_render_surface`、`viewer_read_last_render_result`）、实体创建与写入（node/camera/mesh/material/model/light/tex/clipping/text3d/wide line/wide points/occ）、查询（`viewer_create_picker_api`、`viewer_create_world_derive_query_api` 及 `picker_pick_*`、`world_derive_query_api_get_*`、`query_scene_bounding`）。典型实现如 `create_scene_model`（[c_api/model.rs](../../application/viewer-content-api/src/c_api/model.rs)）——一个 C 调用同时建 `StandardModelEntity`（网格 + occ 材质）与 `SceneModelEntity`（场景归属 + 节点 + payload 外键）。

长期存活对象（API 实例、查询对象、结果列表）由 `Box::leak` 逃逸出 Rust 所有权，配对 `drop_*` 函数用 `Box::from_raw` 回收；查询对象必须在场景修改前销毁（注释警告：否则共享消费者持有的锁会造成死锁）。

## 使用模板

### 模板一：外部宿主最小接入流程

```text
rendiation_init(NULL)                                   // 全局数据库 + 数据模型注册
api = create_viewer_content_api_instance("config.toml") // 或空串走默认配置
surface = viewer_create_surface(api, hwnd, hinstance, w, h)
node = create_node(...); mesh = create_mesh(...); material = create_pbr_mr_material(...)
model = create_scene_model(material, mesh, node, scene) // scene 来自 create_scene()
viewer_render_surface(api, surface)
picker = viewer_create_picker_api(api, surface)
hits = picker_pick_list(picker, api, x, y, tolerance, sort, remove_clipped)
world = viewer_create_world_derive_query_api(api)
query_scene_bounding(world, api, scene, out_bbox, consider_view_dep, consider_infinity, surface)
```

要点：所有内容写入都是同步的全局数据库写入（下一帧渲染与下一次查询自动增量可见）；`render_surface` 与查询调用是同步阻塞的，由宿主自己的线程节奏驱动。

### 模板二：在 Rust 中直接使用 ViewerAPI

C 层只是薄包装，Rust 侧同样可 `ViewerAPI::new` 后直接调 `create_surface` / `render_surface` / `create_query_api`（桌面 viewer 应用 [application/viewer](../../application/viewer) 则是绕过本 crate、直接消费 viewer-content 的 `Viewer`，可作为"宿主内嵌"的完整参考）。

### 模板三：扩展一帧渲染（TopMostStandaloneDraw 模式）

实现 `ViewerFrameRenderingExtension`（[frame_viewport.rs](../../application/viewer-content/src/rendering/frame_viewport.rs)），在 `use_draw_content_on_post_frame` 里用 `FrameCtx` 追加 pass；`render_surface` 目前固定传入 `TopMostStandaloneDraw`，Rust 直接使用时可换成任意扩展实现。

## 延伸阅读

- 查询系统的算法基础：拾取三层抽象与容差换算见 [geometry-query-guide.md](geometry-query-guide.md)；共享派生与两阶段执行见 [query-hook-guide.md](query-hook-guide.md) 与 [hooks-guide.md](hooks-guide.md)。
- 渲染帧组装：pass/attachment 见 [skill-translation/frame-pass-assemble-zh.md](skill-translation/frame-pass-assemble-zh.md)，GPU 组件模型见 [skill-translation/fundamental-gpu-component-model-zh.md](skill-translation/fundamental-gpu-component-model-zh.md)。
- 场景数据模型：实体/组件/外键见 [skill-translation/scene-core-structure-zh.md](skill-translation/scene-core-structure-zh.md)。
- 动态场景 BVH（`SceneBVHResultView` / `use_scene_dynamic_bvh`）：[extension/dynamic-bvh-scene/src/lib.rs](../../extension/dynamic-bvh-scene/src/lib.rs)。
- URI 流式加载与调度（`NoControlStreaming` / `UriDataSourceDyn`）：[utility/uri-streaming](../../utility/uri-streaming)（尚无专项文档）。
