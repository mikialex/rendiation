# Rendiation 文档生成

## to agent

目标是doc/content下构造和组织若干md文档，这些文档的目的是详细的介绍和解释rendiation项目的架构和实现细节，以供一个没有了解过此项目，但是对渲染有基本了解的开发者快速上手此项目。

以下定义不同的任务类型，每个任务都需要spawn子agent，以下子任务描述中的“你”有可能指代子agent，而不是你

定义任务类型「代码研究和文档编写」：要求为一些模块编写文档（文件名guide结尾），比如从某个crate或者特定主题出发，你需要阅读这个crate的实现和相关的实现，其中要特别的了解其中的trait抽象体系，以及trait在其他下游crate的实现细节，要了解api的用户时如何使用的。在编写文档时，你要能直接引用实现（代码跳转链接），你要假设阅读者对实现并不了解，提供循序渐进的引导（但不要在文档中提及此事）。在看代码过程中，如果遇到你不理解的依赖概念你可以按需的了解必要的skill（doc/content/skill-translation），在后续文档编写过程中，你要引用这些skill作为文档阅读的前置资料。当你发现存在和编写目标有重大依赖但是没有任何skill和相关文档时，你需要向parent agent告诉你输出相关的前置文档编写和研究计划，然后停止工作（parent agent需要将任务排在后面完成）。完成文档后需要更新doc/index.md。当你完成文档编写后，你可以向parent agent反馈输出你建议的下一个「代码研究和文档编写」task（如果相关主题还没有被编写, 并且你认为比较有必要）。完成文档后，需要将文档link补充到下面的“新文档编写未检查区内”。在研究过程中，如果发现源代码有明显的逻辑问题（忽略隐患，普通性能问题，必须是明显的逻辑错误），请在下方「文档编写中发现的重要逻辑问题」进行记录。该任务可能可以被并行执行

定义任务类型「文档整合检查」：要求对已有的，未检查过的guide文件在“新文档编写未检查区”内，进行整体检查。需要简单阅读（不需要去follow code link）所有目前的已经检查的文档（可以根据摘要跳过完全没有关系的文档）和未检查的文档。找到其中重复的部分，重复的其他部分修改为对更合理部分的引用。找到其中逻辑冲突矛盾的，这部分自行去check细节，并更新。检查完成的doc移除“新文档编写未检查区”。该任务运行时，不得运行其他输出类型的任务。至少存在3个未检查文档，parent agent才能运行此任务。你（子agent）也可以考虑拆分文档为多个，拆分的文档可以创建子目录。你也可以对多个不同文档内的相同topic进行合理的聚类生成新的文档，以更好帮助用户理解。如果文档过长，且内部包含多个可拆解的单元，也可以拆分。拆分后的文档不必添加到“新文档编写未检查区”。在文档检查完成后，你可以根据对项目的理解向parent提出新的「代码研究和文档编写」计划。完成文档整合后需要更新doc/index.md，子agent还可以适度的对文档摘要进行补充和修改。

你是主管任务分配和调度的主agent，你不需要详细了解每一个doc，只需要了解他们的摘要信息。你需要根据下面的“计划但是未开始任务”任务，自动的spawn子agent干活，注意上述任务的性质和并行串行要求。每一个任务完成后需要在doc/worklog.md按照时间越晚越靠上的顺序，并输出必要的记录信息。你最多允许执行10个任务，当任务完成达到这个数量之后，停止工作，如果没有合适的任务，也可以停下。“计划但是未开始任务”需要实时维护

子agent推荐给你的「代码研究和文档编写」你不一定执行，你要判断合理性，比如检查确认这个topic目前没有doc涵盖才能执行。子agent推荐给你的「代码研究和文档编写」你也可以考虑进行合理拆分。

### 新文档编写未检查区

- [webgpu-buffer-layer-guide](content/webgpu-buffer-layer-guide.md) — WebGPU 缓冲资源层（AbstractBuffer/AbstractStorageAllocator 可插拔分配策略、linear_buffer_array 组合子体系、slab/range 槽位分配器），是 webgpu-hook-utils 稀疏写与 batch-extractor id 池的直接地基
- [lighting-system-guide](content/lighting-system-guide.md) — GPU 光照系统（content/lighting 与 viewer lighting 模块）：LightSystem::prepare/use_lighting 两阶段组织、LightSystemSceneProvider/LightingComputeComponent/LightingComputeInvocation 四层 trait 体系、per-scene 光照 uniform 数组（gpu-gles）、MultiLayerTexturePacker 阴影图集与 PCF/VSM 过滤、五类光源 preparer、前向/延迟光照组件装配与 tonemap

## 计划但是未开始任务

- 「代码研究和文档编写」了解 extension/transform-instanced-model 实例化模型扩展(内部模型渲染器装饰+实例池+source_model_vertices_count 顶点预算跨扩展共享+TransformInstancedMeshPicker 覆写 primitive_index 为实例索引),输出 transform-instanced-model-guide.md(由 extension-indirect-render-patterns-guide 任务建议,同模式最复杂变体,当前无专门文档,可顺带覆盖 view-dependent-transform,低优先级)
- 「代码研究和文档编写」了解 content/texture/gpu-system 纹理系统(TraditionalPerDrawBindingSystem/TexturePool/Bindless 三实现+ gpu-base 纹理输入管线 use_gpu_texture_2ds/use_sampler_gpus、viewer_texture_input、TextureScheduler、GPUBufferImage),输出 texture-system-guide.md(由第三批整合检查建议,中优先级,三实现现分散在 material-indirect-render-guide 与 gles-material-host-render-guide 两处无专文)
- 「代码研究和文档编写」了解 viewer 透明渲染(ViewerTransparentRenderer 的 NaiveAlphaBlend/Loop32OIT/WeightedOIT 三算法),输出 viewer-transparent-render-guide.md(由第三批整合检查建议,低优先级,目前仅 viewer guide 片段提及 transparent.rs)

## 文档编写中发现的重要逻辑问题

- [extension/occ-style-draw-control/src/gles.rs:42](../../extension/occ-style-draw-control/src/gles.rs#L42)：`get_top_most_layer` 的排序 key 写成 `layer & priority`（位与），而同文件 `extract_scene_batch`（:74）的相同模式用的是 `layer | priority`（位或）。`layer` 是 `(layer as u64) << 32`、`priority` 是 u32，两者按位与几乎恒为 0，导致 TopMost 层批的 `sort_by_cached_key` 拿到常数 key、排序失效——TopMost 层的 priority 排序实际不生效（由 viewer-content-frame-pipeline-guide 研究时发现）
- [platform/graphics/webgpu/src/resource/buffer/allocator/slab.rs:38](../../platform/graphics/webgpu/src/resource/buffer/allocator/slab.rs#L38)：`GPUSlatAllocateMaintainer::deallocate_back` 函数体内调用的是 `self.deallocate_back(idx)`（递归调用自己，无终止条件），而按语义（"取出旧值并释放"）应调用 `self.deallocate(idx)`——任何一次调用都会无限递归直至栈溢出。该函数在 shader/ray-tracing/src/backend/wavefront_compute/sbt.rs:201 的 SBT 记录释放路径中被真实调用，一旦触发即崩溃（由 webgpu-buffer-layer-guide 研究时发现）。另：同结构体的 `used_count` 字段从未被 `allocate_value`/`deallocate` 更新，`current_used()` 恒返回 0，当前无下游消费者，暂为隐患
- [application/viewer-content/src/rendering/lighting/light_pass/defer_protocol.rs:377](../../application/viewer-content/src/rendering/lighting/light_pass/defer_protocol.rs#L377)：`PhongSurfaceEncodeDecode::decode` 组装 emissive 时取的是 `(channel_a.w(), channel_b.x(), channel_b.y())`——按同文件 encode（:362-364，channel_a=(diffuse, alpha)、channel_b=(specular, emissive.x)、channel_c=(emissive.yz, alpha, 1)），这分别是 alpha、specular.r、specular.g，而正确的 emissive 应为 `(channel_b.w(), channel_c.x(), channel_c.y())`（对照同文件 `PbrSurfaceEncodeDecode::decode` :299-303 的正确写法）。后果：occ 风格（Phong）材质在 DeferLighting 模式下解码出的 emissive 完全错误（真实 emissive 丢失，混入 alpha 与 specular 分量），且与 decode_alpha（:371 取 channel_a.w）读取的是同一个值（alpha 被用作 emissive 的红通道）（由 lighting-system-guide 研究时发现）
