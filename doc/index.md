# Rendiation 文档

## 架构与实现指南

- [hooks-guide](content/hooks-guide.md) — Hook 模式理解（utility/hook）：状态寻址、作用域、动态/静态阶段与状态生命周期
- [query-hook-guide](content/query-hook-guide.md) — Query-Hook 模式理解（utility/query-hook）：两阶段执行模型、共享计算、唤醒传播与调度基础设施
- [draw-list-guide](content/draw-list-guide.md) — GPU Draw-List 模式理解（shader/draw-list）：DeviceDrawList、多范围绘制、GPU 剔除抽象与流压缩（段前缀和等并行原语见 parallel-compute-primitives-guide）
- [geometry-query-guide](content/geometry-query-guide.md) — 场景几何查询与拾取（scene/geometry-query）：SceneRayQuery/SceneFrustumQuery、模型拾取分层抽象与各场景类型实现
- [batch-extractor-guide](content/batch-extractor-guide.md) — 场景批提取与增量 PSO Key（scene/rendering/batch-extractor）：SceneModelGroupKey 增量计算、GPU id 池两阶段维护、GroupKeyForeignImpl 扩展机制与 occ-style-draw-control 分层绘制
- [viewer-content-api-guide](content/viewer-content-api-guide.md) — 嵌入层 C API（application/viewer-content-api）：多 surface 管理、ViewerAPICx 两阶段执行与独立查询系统（拾取/世界派生/场景包围盒）
- [database-tracing-guide](content/database-tracing-guide.md) — 数据库追踪（utility/database-tracing）：start_tracing 订阅变更事件、TraceIO/TraceReplayTarget 记录格式、文件/回放/转文本全流程与 viewer/C API 使用实例
- [indirect-draw-command-guide](content/indirect-draw-command-guide.md) — 间接绘制命令中间层（scene/rendering/gpu-base/src/mid）：Indexed/NoneIndexed 双分支 builder trait、generator 组件、provider 三合一抽象，以及 MultiDrawIndirectCount 不可用时经 webgpu-midc-downgrade 段前缀和降级为单段 indirect 的完整机制
- [attribute-mesh-indirect-render-guide](content/attribute-mesh-indirect-render-guide.md) — 属性网格间接渲染（scene/rendering/gpu-indirect/src/shape/attribute）：attribute mesh 顶点池化切片、AttributeMeshMeta 两跳间接寻址、宿主/compute 双通道间接命令生成与 MIDC 降级细节
- [attribute-mesh-lod-guide](content/attribute-mesh-lod-guide.md) — 场景模型 LOD（scene/rendering/attribute-mesh-lod）：CPU 网格简化生成多级合并索引缓冲、level_meta/lod_levels 元数据组织、设备侧投影误差级别选择与宿主侧只画根级、与 view_dependent_transform 的耦合及 trait 包装式接入
- [occlusion-culling-guide](content/occlusion-culling-guide.md) — 两遍 GPU 遮挡剔除（scene/rendering/occlusion-culling）：上一帧可见性拆分 occluder/subject 遍、深度金字塔遮挡测试与帧间可见性状态闭环
- [material-indirect-render-guide](content/material-indirect-render-guide.md) — 材质间接渲染（scene/rendering/gpu-indirect/src/material）：三类标准材质参数与纹理句柄双缓冲投影、材质 id 注入、GPUTextureBindingSystem 间接采样与 alpha_mode 驱动的 PSO/批分组
- [parallel-compute-primitives-guide](content/parallel-compute-primitives-guide.md) — GPU 并行原语（shader/parallel-compute）：流压缩、Kogge-Stone 段前缀和与 radix sort 的组件模型与算法细节（draw-list 剔除、MIDC 降级、遮挡剔除的算法底座）
- [gpu-indirect-batch-collector-guide](content/gpu-indirect-batch-collector-guide.md) — 间接绘制链路组织（scene/rendering/gpu-indirect + scheduler）：use_make_scene_batch_pass_content 二次分类与 pass 组装、RenderBatchCollector 帧末批次收集及整条链路在 viewer 中的装配
- [task-graph-guide](content/task-graph-guide.md) — GPU 任务图执行运行时（shader/task-graph）：任务组/任务池/状态机、轮询执行引擎、bump 分配器与 future 异步组合模型（wavefront 光线追踪的调度底座）
- [extension-indirect-render-patterns-guide](content/extension-indirect-render-patterns-guide.md) — extension 间接绘制实现模式（wide-line/wide-styled-points/text-3d/cell-mesh）：builder + key + picker 三件套、几何池/参数行/索引映射渲染器骨架、顶点展开差异与拾取容差
- [gles-material-host-render-guide](content/gles-material-host-render-guide.md) — GLES 材质与模型 host 渲染（scene/rendering/gpu-gles）：GLESModelMaterialRenderImpl 等 trait 体系、每材质实体 uniform 增量维护、setup_tex 纹理直绑与传统每绘制绑定、use_gles_scene_model_renderer 逐模型组件组装与双后端分工
- [webgpu-hook-utils-guide](content/webgpu-hook-utils-guide.md) — GPU Hook 基建（platform/graphics/webgpu-hook-utils）：QueryGPUHookCx 的 Update/CreateRender 两阶段 GPU 维护、稀疏写入 SparseBufferWritesSource 与可增长存储缓冲、GrowableRangeAllocator 批分配封装及 DataChangeGPUExt 统一写入通道
- [webgpu-buffer-layer-guide](content/webgpu-buffer-layer-guide.md) — WebGPU 缓冲资源层（platform/graphics/webgpu/src/resource/buffer）：AbstractBuffer/AbstractStorageAllocator 可插拔分配策略（默认/texture-as-buffer/合并缓冲）、linear_buffer_array 组合子体系（with_direct_resize/with_vec_backup/with_default_grow_behavior 等）与 slab/range 槽位分配器，是 webgpu-hook-utils 稀疏写与 batch-extractor id 池的直接地基
- [viewer-content-frame-pipeline-guide](content/viewer-content-frame-pipeline-guide.md) — viewer 帧流水线装配（application/viewer-content）：RenderingRoot/Viewer3dRenderingCtx/ViewerBatchExtractor 生命周期、Update/CreateRender 两阶段驱动、device/Gles 双后端分流、frustum/遮挡剔除与光照在帧内的装配及逐视口 pass 序列
- [lighting-system-guide](content/lighting-system-guide.md) — GPU 光照系统（content/lighting + viewer lighting 模块）：LightSystem::prepare/use_lighting 两阶段组织、四层光照计算 trait 体系、per-scene 光照 uniform 数组、MultiLayerTexturePacker 阴影图集与 PCF/VSM 过滤、五类光源 preparer、前向/延迟光照组件装配与 tonemap

## 现有skill的中文版本翻译

- [database-schema-zh](content/skill-translation/database-schema-zh.md) — 类型安全关系数据库层（utility/database）：声明实体/组件/外键、注册 schema、CRUD、存储后端与钩子系统
- [frame-pass-assemble-zh](content/skill-translation/frame-pass-assemble-zh.md) — 多通道渲染帧组装：pass()/attachment()/render_ctx()/by()、FrameCtx、PassContent
- [fundamental-gpu-component-model-zh](content/skill-translation/fundamental-gpu-component-model-zh.md) — 可组合 GPU 组件模型：RenderComponent、ShaderHashProvider、ShaderPassBuilder、便捷包装器
- [query-system-zh](content/skill-translation/query-system-zh.md) — 增量查询系统（utility/query）：Query/MultiQuery、组合算子、双查询增量模型与 fanout 传播
- [rendiation-algebra-zh](content/skill-translation/rendiation-algebra-zh.md) — 数学库（math/algebra）：Vec/Mat/Quat、Scalar trait、泛型常量构造、SpaceEntity
- [scene-core-structure-zh](content/skill-translation/scene-core-structure-zh.md) — 场景数据模型（scene/core）：实体类型、组件、场景图层级、SceneWriter/SceneReader
- [shader-edsl-binding-and-typed-container-zh](content/skill-translation/shader-edsl-binding-and-typed-container-zh.md) — 强类型 GPU 资源容器及其绑定：bind_by 与 pass 侧 bind
- [shader-edsl-compute-zh](content/skill-translation/shader-edsl-compute-zh.md) — 计算管线：管线构建、GPU 单元测试、工作组共享/私有内存、屏障、计算内置量
- [shader-edsl-core-zh](content/skill-translation/shader-edsl-core-zh.md) — 着色器 EDSL 核心语言：Node&lt;T&gt;、着色器结构体、内存布局、控制流、纹理与原子操作
- [shader-edsl-graphics-zh](content/skill-translation/shader-edsl-graphics-zh.md) — 图形管线：顶点/片元阶段、语义、资源绑定、渲染目标与常见配方
- [viewer-scene-building-zh](content/skill-translation/viewer-scene-building-zh.md) — viewer 场景构建：ParametricSurface 网格生成、材质创建、灯光与测试内容模块
