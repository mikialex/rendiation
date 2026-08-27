# Rendiation 间接绘制命令基础设施指南（scene/rendering/gpu-base/src/mid）

本文梳理 [scene/rendering/gpu-base/src/mid](../../scene/rendering/gpu-base/src/mid/mod.rs) 的间接绘制命令（indirect draw command）基础设施：如何把"按子列表分组的场景模型 id 流"变成 GPU 侧可提交的间接绘制命令，以及当平台不支持 MultiDrawIndirectCount（MIDC）时如何整体降级为多次单 draw indirect。本模块位于间接渲染管线的中间层——上游是批提取与 GPU 剔除产出的 `DeviceDrawList`（见 [draw-list-guide.md](draw-list-guide.md)），下游是 webgpu 的 `DrawCommand` 提交与顶点着色器。渲染实现（材质、网格存储等）的具体细节见 gpu-indirect 相关文档，这里只聚焦 trait 抽象与降级机制。

## 前置阅读

间接绘制命令依赖 GPU 绘制列表、计算管线与 GPU 组件模型，建议先了解：

| 文档 | 内容 |
| --- | --- |
| [draw-list-guide.md](draw-list-guide.md) | `DeviceDrawList`（id pool + 子列表范围）、`MultiRangeDispatchInfo`、GPU 剔除与流压缩 |
| [skill-translation/shader-edsl-compute-zh.md](skill-translation/shader-edsl-compute-zh.md) | 计算管线构建、内置计算 ID、流压缩（前缀和） |
| [skill-translation/fundamental-gpu-component-model-zh.md](skill-translation/fundamental-gpu-component-model-zh.md) | `ShaderHashProvider` / `ShaderPassBuilder` / `GraphicsShaderProvider` 与管线哈希 |
| [skill-translation/shader-edsl-graphics-zh.md](skill-translation/shader-edsl-graphics-zh.md) | 顶点着色器阶段与语义注册 |
| [skill-translation/frame-pass-assemble-zh.md](skill-translation/frame-pass-assemble-zh.md) | 渲染帧组装与 pass 内容源 |
| [batch-extractor-guide.md](batch-extractor-guide.md) | 场景批提取：`DeviceSceneModelDrawList` 如何按 PSO key 分组（本模块的输入） |
| [attribute-mesh-indirect-render-guide.md](attribute-mesh-indirect-render-guide.md) | 标准属性网格对本模块 builder / provider 的完整实现（典型下游实现示例） |

## 模式概览

间接渲染中，剔除后的场景模型 id 存在 GPU 的 id pool 里，按"共享同一条管线"的语义分成若干子列表（sub-list），每个子列表只占 pool 的一段区域。绘制前需要回答一个问题：**每个 id 应该画什么？** 顶点数、索引偏移、实例数这些参数只存在于网格数据里，而网格数据又在 GPU 的存储 buffer 里——所以"id → draw command"必须在 GPU 上完成。本模块给出这套转换的抽象：

- **双分支命令构建器**：`DrawCommandBuilder::Indexed / NoneIndexed` 覆盖有索引（`DrawIndexedIndirectArgsStorage`）与无索引（`DrawIndirectArgsStorage`）两种绘制。每个分支的 builder trait 同时提供 host 侧访问（`draw_command_host_access`，供 host-driven 路径直接拿一个 `DrawCommand`）与 compute 侧生成（`build_invocation`，供 GPU 逐 id 生成命令）两种表达。
- **统一组装入口**：`use_and_create_default_indirect_draw_provider` 把"场景模型 id 流组件 + 命令构建器"组合成一个 compute pass（一个线程一个 id），把生成的命令写进 `INDIRECT` 用途的存储 buffer，再按子列表切成 view，产出每子列表一个的 `IndirectDrawProvider`。
- **空 drawcall 约定**：网格尚未分配（数据未加载完）时，生成器必须产出计数为零的空 drawcall 而不是跳过该 id——用一次多余的空命令换掉一次流压缩 pass，因为后者更昂贵。
- **provider 三合一**：`IndirectDrawProvider = ShaderHashProvider + ShaderPassBuilder + 顶点注入`。渲染端把 provider 当普通 `RenderComponent` 使用，命令本身通过 `draw_command()` 取出交给绘制提交。
- **MIDC 降级**：平台不支持 `MultiDrawIndirectCount`（feature 缺失，或 Dx12 上的已知 bug）时，把"每子列表一条多段绘制"降级为"每子列表一条单段 indirect + 一个前缀和 helper"。顶点着色器通过 helper 二分查找恢复"当前顶点属于哪个子命令、子命令内偏移是多少"，行为与原多段绘制一致。

为什么需要这个中间层，而不是让每个渲染实现自己写生成逻辑？三个原因：

- **生成命令需要访问网格数据**。顶点数、索引偏移只存在于 GPU 侧的网格元数据表里，host 侧读不到（或读回太贵），生成必须发生在 GPU 上、且绑定与生成逻辑一一对应——这套绑定正是 builder 的 `build_invocation` / `bind` 两个方法承载的。
- **命令数量由剔除动态决定**。id 流经剔除后存活数每帧变化，生成 pass 用间接派发（dispatch size 由 GPU 侧算出），这要求"id 流"与"命令池"按统一的子列表布局组织——布局知识沉淀在本模块而不是各渲染实现里。
- **同一数据源要表达两次**。host-driven（CPU 每帧遍历）与 device 剔除（纯 GPU）两条渲染路径都要从同一份网格数据生成命令，若各自实现会漂移（如 u16 索引展开、分配失败标记的语义）。builder trait 强制两侧共享同一个实现对象与同一批 buffer。

## 核心概念

| 概念 | 定义位置 | 说明 |
| --- | --- | --- |
| `DrawCommand` | [platform/graphics/webgpu/src/pass.rs:421](../../platform/graphics/webgpu/src/pass.rs#L421) | webgpu 绘制命令枚举：`Indexed` / `Array`（host 直发）、`Indirect` / `MultiIndirect` / `MultiIndirectCount`（GPU 间接） |
| `DrawCommandBuilder` | [scene/rendering/gpu-base/src/mid/mod.rs:12](../../scene/rendering/gpu-base/src/mid/mod.rs#L12) | 命令构建器二分支枚举：`Indexed(Box<dyn IndexedDrawCommandBuilder>)` / `NoneIndexed(...)` |
| `IndexedDrawCommandBuilder` | [mid/indexed.rs:3](../../scene/rendering/gpu-base/src/mid/indexed.rs#L3) | 有索引命令构建器 trait：host 访问 + compute invocation + 资源绑定 |
| `NoneIndexedDrawCommandBuilder` | [mid/none_indexed.rs:3](../../scene/rendering/gpu-base/src/mid/none_indexed.rs#L3) | 无索引对应物 |
| `IndexedDrawCommandBuilderInvocation` | [mid/indexed.rs:14](../../scene/rendering/gpu-base/src/mid/indexed.rs#L14) | 着色器侧执行体：`generate_draw_command(draw_id) -> Node<DrawIndexedIndirectArgsStorage>`，要求 mesh 未分配时生成空 drawcall |
| `NoneIndexedDrawCommandBuilderInvocation` | [mid/none_indexed.rs:37](../../scene/rendering/gpu-base/src/mid/none_indexed.rs#L37) | 无索引对应物 |
| `IndexedDrawCommandGeneratorComponent` | [mid/indexed.rs:47](../../scene/rendering/gpu-base/src/mid/indexed.rs#L47) | 生成器组件：`scene_models`（id 流，即 `DeviceDrawList` 的 `ComputeComponent<Node<Vec2<u32>>>`）+ generator，实现 `ComputeComponent` |
| `DrawCommandGeneratorComponent` | [mid/none_indexed.rs:47](../../scene/rendering/gpu-base/src/mid/none_indexed.rs#L47) | 无索引对应物 |
| `IndirectDrawProvider` | [mid/mod.rs:30](../../scene/rendering/gpu-base/src/mid/mod.rs#L30) | 间接绘制提供者：`ShaderHashProvider + ShaderPassBuilder`，外加顶点注入源与 `draw_command()` |
| `IndirectBatchInvocationSource` | [mid/mod.rs:38](../../scene/rendering/gpu-base/src/mid/mod.rs#L38) | 顶点侧"当前调用属于哪个场景模型"的注入点 |
| `IndirectDrawProviderAsRenderComponent` | [mid/mod.rs:43](../../scene/rendering/gpu-base/src/mid/mod.rs#L43) | provider 的 `RenderComponent` 适配器，顶点阶段注册 `LogicalRenderEntityId` |
| `use_and_create_default_indirect_draw_provider` | [mid/mod.rs:84](../../scene/rendering/gpu-base/src/mid/mod.rs#L84) | 默认组装：compute 生成命令 → 子列表切片 → provider 列表 |
| `MultiIndirectDrawBatch` | [mid/mod.rs:318](../../scene/rendering/gpu-base/src/mid/mod.rs#L318) | 原生 MIDC 路径的 provider：命令 buffer view + count view |
| `MIDCDowngradeBatch` | [mid/midc_downgrade.rs:6](../../scene/rendering/gpu-base/src/mid/midc_downgrade.rs#L6) | 降级路径的 provider：helper + 降级命令 + 内部 provider 组合 |
| `DrawIndexedIndirectArgsStorage` / `DrawIndirectArgsStorage` | [platform/graphics/webgpu/src/indirect.rs:6](../../platform/graphics/webgpu/src/indirect.rs#L6) | GPU 侧间接参数结构体（vertex_count / instance_count / base_index 等），`std430` 布局 |
| `StorageDrawCommands` | [platform/graphics/webgpu/src/indirect.rs:80](../../platform/graphics/webgpu/src/indirect.rs#L80) | 命令池的抽象 buffer 枚举（Indexed / NoneIndexed），提供 `is_index`、`cmd_capacity_count`、`indirect_buffer` |
| `DowngradeMultiIndirectDrawCountHelper` | [platform/graphics/webgpu-midc-downgrade/src/draw_helper.rs:3](../../platform/graphics/webgpu-midc-downgrade/src/draw_helper.rs#L3) | 降级后顶点侧恢复"子命令下标"的 helper（前缀和 + 命令池 + 真实计数） |
| `use_downgrade_multi_indirect_draw_count_list_pool` | [platform/graphics/webgpu-midc-downgrade/src/lib.rs:46](../../platform/graphics/webgpu-midc-downgrade/src/lib.rs#L46) | 整个降级管线：段前缀和 → 单段 indirect 参数 + helper 数据 |
| `require_midc_downgrade` | [platform/graphics/webgpu-midc-downgrade/src/lib.rs:20](../../platform/graphics/webgpu-midc-downgrade/src/lib.rs#L20) | 降级判定：强制开关 / Dx12（wgpu#7974）/ 缺 `MULTI_DRAW_INDIRECT_COUNT` feature |
| `MidcDowngradeWrapperForIndirectMeshSystem` | [platform/graphics/webgpu-midc-downgrade/src/mesh_sys_wrapper.rs:3](../../platform/graphics/webgpu-midc-downgrade/src/mesh_sys_wrapper.rs#L3) | 降级模式下顶点阶段的索引覆写（u16 从 u32 池解包） |
| `IndirectDrawProviderCreator` / `DrawCommandBuilderCreator` | [scene/rendering/gpu-indirect/src/scene.rs:114](../../scene/rendering/gpu-indirect/src/scene.rs#L114) | gpu-indirect 侧的两个下游 trait：按实现分类创建 provider / 按代表实体构造 builder |

## 分层动机与数据流

先看完整数据流，再逐层展开：

```text
batch-extractor: (scene, key) 分组 → DeviceDrawList(id pool + 子列表范围 + 前缀和)
  └─ GPU 剔除 + 流压缩: 存活 id 压紧, 子列表 count 回写
       └─ gpu-indirect scene.rs: 按 impl key 把子列表分类
            └─ use_compute_selected_sub_list_dispatch_info
                 └─ use_and_create_default_indirect_draw_provider(list, compacted_info, builder, cx, enable_downgrade)
                      ├─ compute pass: 每线程一个 id
                      │    ├─ scene_models.invocation_logic → (sm_id, sub_list_index, valid)
                      │    └─ generator.generate_draw_command(sm_id) → 写 draw command buffer
                      ├─ 按子列表切 buffer view (create_pool_views, 对齐 storage offset)
                      ├─ count view ← 子列表范围描述符的 count 字段 (流压缩回写)
                      ├─ 每子列表一个 MultiIndirectDrawBatch (命令 view + count view)
                      └─ enable_downgrade 时再包一层 MIDCDowngradeBatch
                           └─ 段前缀和 → 每子列表一条 DrawIndirectArgsStorage + helper(前缀和/计数/命令池)
                                └─ IndirectDrawProvider
                                     └─ render_indirect_batch_models
                                          └─ RenderMethod::TraditionalDraw(models.draw_command())
```

分层动机：

- **builder 与 invocation 分离**。"如何从网格数据生成命令"的知识要表达两次：host 侧（host-driven 路径现场生成 `DrawCommand`）与 device 侧（compute shader 生成间接参数）。两者共享同一个 builder 对象与同一批 buffer 绑定，只是呈现形式不同，避免两套独立实现漂移。
- **生成与切片分离**。compute pass 把命令写入一个"总容量 = 所有子列表容量之和"的大 buffer（连续化输出偏移）；组装时再按子列表切 view。这样命令池一次生成、多份视图复用，且每份视图天然满足 storage buffer offset 对齐。
- **空 drawcall 约定**（[mid/indexed.rs:15](../../scene/rendering/gpu-base/src/mid/indexed.rs#L15)）：实现方必须为未分配 mesh 生成空命令，而不是让该 id 缺席——否则需要第二次流压缩 pass 才能保持"命令数组与 id 数组一一对应"，成本更高。零顶点命令会被 GPU 直接跳过。
- **provider 自包含**。provider 同时承载管线哈希（PSO 缓存键）、pass 绑定（`setup_pass`）与顶点注入（invocation source），渲染端拿一个 `&dyn IndirectDrawProvider` 就能完成"哈希 → 建 PSO → 绑定 → 提交"全流程。
- **降级在组装层完成**。降级把"一条多段命令 + 每段 count"变成"多条单段命令 + 前缀和"。渲染端与 PSO 均不感知两种模式的差异（顶点侧通过统一注册的语义读取 id），这是它被放在 provider 创建处的原因。

## DrawCommand 与 DrawCommandBuilder

`DrawCommand`（[webgpu/src/pass.rs:421](../../platform/graphics/webgpu/src/pass.rs#L421)）是提交层的命令枚举，其中间接绘制有三档：

- `Indirect { indirect_buffer, indexed }`：单条间接绘制。
- `MultiIndirect { indirect_buffer, indexed, count }`：固定段数的多段绘制。
- `MultiIndirectCount { indirect_buffer, indirect_count, indexed, max_count }`：段数由另一个 buffer（count buffer）在 GPU 上决定的多段绘制——这正是剔除链路的出口：count buffer 指向子列表范围描述符里的 `count` 字段（[multi_range.rs:39](../../shader/draw-list/src/multi_range.rs#L39) 的 `create_indirect_count_views` 切出的 4 字节 view），剔除流压缩回写它，绘制端无需 CPU 干预。

`DrawCommandBuilder`（[mid/mod.rs:12](../../scene/rendering/gpu-base/src/mid/mod.rs#L12)）是"命令生成器"的 boxed 枚举，两个分支的结构完全同构。以 Indexed 为例，trait（[mid/indexed.rs:3](../../scene/rendering/gpu-base/src/mid/indexed.rs#L3)）有三个方法：

- `draw_command_host_access(id) -> Option<DrawCommand>`：host 侧。给定场景模型实体，读宿主侧数据（地址表、索引范围等）直接构造 `DrawCommand`。返回 `None` 表示数据未就绪。
- `build_invocation(cx) -> Box<dyn IndexedDrawCommandBuilderInvocation>`：compute 侧。在计算管线构建时把同一批 buffer 绑进 shader，返回着色器侧执行体。
- `bind(builder)`：把同一批 buffer 绑进 compute pass 的 bind group。

两个 trait 细节值得注意：

- builder 需要 `DynClone`（`dyn_clone::clone_trait_object!`，[mid/indexed.rs:12](../../scene/rendering/gpu-base/src/mid/indexed.rs#L12)）：provider 创建发生在计算上下文的 scope 内，builder 要被克隆进生成器组件、按需复用，而 trait 对象默认不可克隆。
- `Box<dyn IndexedDrawCommandBuilder>` 本身也实现 `IndexedDrawCommandBuilder`（[mid/indexed.rs:30](../../scene/rendering/gpu-base/src/mid/indexed.rs#L30)，纯委托）：这让"包装式扩展"（LOD、实例化模型）可以安全地持有内部 builder 的 trait 对象并原样透传三个方法。
- `DrawCommandBuilder::draw_command_host_access`（[mid/mod.rs:19](../../scene/rendering/gpu-base/src/mid/mod.rs#L19)）是枚举上的便捷分发，host-driven 路径拿 `&DrawCommandBuilder` 就能访问 host 命令，不必先 match 分支。

## compute 侧生成：generator 组件与 invocation

`IndexedDrawCommandGeneratorComponent`（[mid/indexed.rs:47](../../scene/rendering/gpu-base/src/mid/indexed.rs#L47)）实现 `ComputeComponent<IndexedDrawTuple>`，其中 `IndexedDrawTuple = (Node<DrawIndexedIndirectArgsStorage>, Node<u32>)`，即"命令 + 子列表下标"：

- `scene_models: Box<dyn ComputeComponent<Node<Vec2<u32>>>>`：id 流组件。`DeviceDrawList` 正是这种组件（每个线程输出 `(scene_model_id, sub_list_index)`，见 draw-list-guide 的 `list_access.rs`）。
- `build_shader` 返回 `DrawCommandGeneratorInvocation`（[mid/indexed.rs:95](../../scene/rendering/gpu-base/src/mid/indexed.rs#L95)）：`invocation_logic` 先取 `(id, valid)`，`valid` 时用 `make_local_var` 暂存生成结果，最后输出 `(命令, 子列表下标, valid)`。命令生成发生在 `if_by(valid)` 内，未剔除的 id 产生零值命令但不被写出。

`use_and_create_default_indirect_draw_provider`（[mid/mod.rs:84](../../scene/rendering/gpu-base/src/mid/mod.rs#L84)）把整条流水线组装起来：

- `prepare_gpu_sub_list_out_ranges`（[mid/mod.rs:71](../../scene/rendering/gpu-base/src/mid/mod.rs#L71)）：把各子列表容量连续累加，得到输出端连续化偏移（compacted 布局）。
- 创建总容量为所有子列表容量之和的 `INDIRECT` 用途存储 buffer 作为命令池。
- `dispatch_size = generator.compute_work_size(cx)`（[parallel-compute/src/abstract_component.rs:63](../../shader/parallel-compute/src/abstract_component.rs#L63)）：按剔除后的存活数算间接派发尺寸——因此生成 pass 用 `dispatch_workgroups_indirect`，剔除结果动态决定线程数。
- 生成 pass 的写回逻辑（[mid/mod.rs:125-133](../../scene/rendering/gpu-base/src/mid/mod.rs#L125)）：

```rust
let ((cmd, list_index), valid) = generator.invocation_logic(builder.global_invocation_id());
if_by(valid, || {
  let range_write_offset = output_ranges.index(list_index).load().x();
  let range_base_offset = input_ranges.index(list_index).count_prefix_sum().load();
  let range_relative_index = builder.global_invocation_id().x() - range_base_offset;
  let write_index = range_relative_index + range_write_offset;
  draw_command_buffer.index(write_index).store(cmd);
});
```

`input_ranges` 是子列表范围描述符（前缀和即该子列表在原始 pool 的起点，保证线程定位准确），`output_ranges` 是输出端连续化偏移——两者分离让"读 pool 的下标"与"写命令池的下标"使用不同的布局，下游 view 切片只需按输出端偏移对齐。`if_by(valid)` 保护写入，被剔除的线程不污染命令池。

- 命令池转只读视图后，`create_pool_views`（[mid/mod.rs:294](../../scene/rendering/gpu-base/src/mid/mod.rs#L294)）按输出偏移逐子列表切 view（偏移以 `min_storage_buffer_offset_alignment` 对齐断言）。
- count view 来自 `list.create_indirect_count_views()`——即子列表范围描述符的 count 字段（[multi_range.rs:39](../../shader/draw-list/src/multi_range.rs#L39)），由剔除流压缩回写。

## 原生 MIDC 路径：MultiIndirectDrawBatch

`MultiIndirectDrawBatch`（[mid/mod.rs:318](../../scene/rendering/gpu-base/src/mid/mod.rs#L318)）持有"本子列表的命令池 view + count view"，作为 `IndirectDrawProvider`：

- `draw_command()` 返回 `DrawCommand::MultiIndirectCount`（[mid/mod.rs:339](../../scene/rendering/gpu-base/src/mid/mod.rs#L339)）：`max_count = cmd_capacity_count()`，`indirect_count` 指向剔除回写的 count view。
- `create_indirect_invocation_source` 返回一个空实现：顶点侧直接 `query::<VertexInstanceIndex>()` 作为场景模型 id——因为生成命令时 `base_instance` 被写成 `draw_id`（sm id，见下文实现示例），原生 MIDC 下顶点实例索引天然就是 id。

## 顶点注入：IndirectDrawProviderAsRenderComponent

`IndirectDrawProviderAsRenderComponent`（[mid/mod.rs:43](../../scene/rendering/gpu-base/src/mid/mod.rs#L43)）把 provider 适配成 `GraphicsShaderProvider + ShaderPassBuilder`：`build`（[mid/mod.rs:59](../../scene/rendering/gpu-base/src/mid/mod.rs#L59)）在顶点阶段调用 `create_indirect_invocation_source`，把 `current_invocation_scene_model_id` 注册为 `LogicalRenderEntityId` 与 `RootLogicalRenderEntityId`，再调用 `extra_register`（默认空）。之后渲染实现（节点变换、材质等）就可以按语义查询当前实体 id。

## MIDC 降级机制

### 何时降级

`require_midc_downgrade`（[webgpu-midc-downgrade/src/lib.rs:20](../../platform/graphics/webgpu-midc-downgrade/src/lib.rs#L20)）：强制开关（`force_midc_downgrade`）、Dx12 后端（wgpu#7974 的已知问题）、或设备缺少 `MULTI_DRAW_INDIRECT_COUNT` feature（如部分移动端/WebGL 后端）时返回 true。viewer 里 force 开关来自 `using_host_driven_indirect_draw` 与纹理作存储 buffer 的配置（[attribute/mod.rs:52](../../scene/rendering/gpu-indirect/src/shape/attribute/mod.rs#L52)）。

### 降级管线

`use_downgrade_multi_indirect_draw_count_list_pool`（[lib.rs:46](../../platform/graphics/webgpu-midc-downgrade/src/lib.rs#L46)）在单个计算批次里完成全部子列表的降级：

- 先做**段前缀和**（`ListPoolVertexCountSource::use_segmented_prefix_scan_kogge_stone`）：对命令池里所有子命令的 vertex_count 分段扫描，得到每个顶点在全池范围内的含首前缀和。
- 一个小 pass 写派发尺寸：`(总存活顶点数 + 子列表数) / 256` 个 workgroup，用间接派发驱动主 pass。
- 主 pass 每线程一个顶点：计算该顶点所在子列表、子列表内相对前缀和，写入该子列表的**排他前缀和段**（容量 +1，最后一项是总顶点数）；每个子列表的最后一个线程额外写：本子列表总顶点数（对齐后的 `aligned_counts`）与一条 `DrawIndirectArgsStorage { vertex_count: 总顶点数, instance_count: 1, ... }`。
- 结果：每子列表一条**单段无索引 indirect**（`DrawCommand::Indirect { indexed: false }`），加上一个 `DowngradeMultiIndirectDrawCountHelper`（[draw_helper.rs:3](../../platform/graphics/webgpu-midc-downgrade/src/draw_helper.rs#L3)），其中 `sub_draw_range_start_prefix_sum` 是该子列表的排他前缀和段（逐子命令的起点）、`draw_commands` 是该子列表的命令池 view、`draw_count` 是真实命令数。
- 空子列表处理：命令池跨帧复用时残留上一帧数据，因此先 `clear_buffer` 清零输出，避免 count=0 的子列表携带陈旧命令（[lib.rs:87-94](../../platform/graphics/webgpu-midc-downgrade/src/lib.rs#L87) 注释）——主 pass 里 `local_idx == count-1` 在 count=0 时溢出为 `u32::MAX`，最后一个线程永远不会触发，只能靠清零兜底。
- 单子列表特例 `use_downgrade_multi_indirect_draw_count`（[lib.rs:328](../../platform/graphics/webgpu-midc-downgrade/src/lib.rs#L328)）把一条 `MultiIndirectCount` 包成单子列表的 `MIDCListPoolInput` 后复用同一管线。

降级路径有一条重要约束（[lib.rs:40-44](../../platform/graphics/webgpu-midc-downgrade/src/lib.rs#L40) 注释）：**子命令不支持 `instance_count > 1`**——单段绘制的顶点数来自命令池里各子命令 `vertex_count` 的总和，实例展开必须提前烘焙进 vertex_count（实例化模型正是这样做的：[draw_cmd.rs:42-48](../../extension/transform-instanced-model/src/indirect_draw/draw_cmd.rs#L42) 把 `vertex_count * instance_count` 写进命令）。另外降级后的间接参数是"整个子列表画一次"，`base_vertex = 0`、实例数为 1，真正的"多实例"语义由顶点侧从索引池重新计算。

### 顶点侧恢复：helper 二分查找

降级后顶点着色器拿到的是"子列表内的全局顶点号"，必须恢复"子命令下标 + 子命令内偏移"。`DowngradeMultiIndirectDrawCountHelperInvocation::current_invocation_scene_model_id`（[draw_helper.rs:48](../../platform/graphics/webgpu-midc-downgrade/src/draw_helper.rs#L48)）：

- 以 `VertexIndex` 为 key，对排他前缀和段做**二分查找**（[draw_helper.rs:67](../../platform/graphics/webgpu-midc-downgrade/src/draw_helper.rs#L67)），找到该顶点属于哪条子命令。
- 从子命令读出 `base_index` / `base_vertex` 与 `base_instance`，注册两个专用语义：`VertexIndexForMIDCDowngradeBaseIndex`（子命令起始偏移）与 `VertexIndexForMIDCDowngradeRelativeInSubDraw`（子命令内相对顶点号），并把 `base_instance` 注册为 `VertexInstanceIndex`——这样下游查询场景模型 id 的代码**无需感知降级**。

这两个专用语义由 `only_vertex!` 宏声明（[webgpu-midc-downgrade/src/lib.rs:17](../../platform/graphics/webgpu-midc-downgrade/src/lib.rs#L17)）。注意原多段绘制中 `base_instance` 就是子命令下标本身，原生 MIDC 路径直接 `query::<VertexInstanceIndex>()`；降级路径里 helper 的二分查找重建了这个值——两条路径对下游暴露的顶点语义完全一致，这正是 `MIDCDowngradeBatch` 顶部注释 "assuming T using VertexInstanceIndex as draw id"（[midc_downgrade.rs:5](../../scene/rendering/gpu-base/src/mid/midc_downgrade.rs#L5)）的含义。

### 索引覆写

有索引的网格在降级后变成单段无索引绘制，索引数据通过存储 buffer 暴露。`MidcDowngradeWrapperForIndirectMeshSystem`（[mesh_sys_wrapper.rs:3](../../platform/graphics/webgpu-midc-downgrade/src/mesh_sys_wrapper.rs#L3)）作为可选的 `RenderComponent` 参与顶点阶段：用 `base_index + relative` 从索引池读真实索引并覆写 `VertexIndex`；u16 索引两两打包在 u32 里，按 `relative % 2` 解包低/高半字。`render_indirect_batch_models` 在降级模式下把它插进绑定数组（[scene_model.rs:118](../../scene/rendering/gpu-indirect/src/scene_model.rs#L118)）。

### MIDCDowngradeBatch

`MIDCDowngradeBatch<T>`（[mid/midc_downgrade.rs:6](../../scene/rendering/gpu-base/src/mid/midc_downgrade.rs#L6)）是降级后的 provider 组合：`helper`（降级数据）+ `cmd`（降级后的 `DrawCommand::Indirect`）+ `internal`（原本的 `MultiIndirectDrawBatch`）。它的 `ShaderHashProvider` 合并 helper 与 internal 的哈希（helper 的哈希只有 `is_index` 一位，[draw_helper.rs:9](../../platform/graphics/webgpu-midc-downgrade/src/draw_helper.rs#L9)——indexed 与非 indexed 的降级命令池布局不同）；顶点注入委托给 helper 的二分查找，`extra_register` 透传给 internal。注意 `draw_command()` 返回的是降级后的单段命令，internal 的 `MultiIndirectCount` 不再参与提交——它只作为管线哈希与资源绑定的载体。

## 下游消费：gpu-indirect

[scene.rs:167](../../scene/rendering/gpu-indirect/src/scene.rs#L167) 的 `use_make_scene_batch_pass_content` 是消费入口：按 `impl_select_ids` 的 `get_impl_distinguish_key_by_impl_select_id` 把子列表按渲染实现分类，对每类实现用 `use_compute_selected_sub_list_dispatch_info`（[scene.rs:270](../../scene/rendering/gpu-indirect/src/scene.rs#L270)）重算"原始偏移 / 连续化偏移"两套派发信息（后者供输出布局与降级切片使用）并创建 provider——组织链路的完整实现见 [gpu-indirect-batch-collector-guide.md](gpu-indirect-batch-collector-guide.md) 的「use_make_scene_batch_pass_content 的实现」，这里只补充本模块相关的消费细节：

- `IndirectDrawProviderCreator`（[scene.rs:114](../../scene/rendering/gpu-indirect/src/scene.rs#L114)）与 `DrawCommandBuilderCreator`（[scene.rs:127](../../scene/rendering/gpu-indirect/src/scene.rs#L127)）由渲染器实现：前者做"按代表实体建 provider"，后者做"按代表实体选命令构建器"。实现类（`AttributeMeshIndirectRenderer`、`AttributeLODMeshIndirectRenderer`、宽线/宽点/文字/单元网格/实例化模型等）以 `IndirectModelRenderImpl`（[std_model.rs:5](../../scene/rendering/gpu-indirect/src/std_model.rs#L5)）聚合进 `IndirectPreferredComOrderRenderer`。
- 提交侧：`render_indirect_batch_models`（[scene_model.rs:96](../../scene/rendering/gpu-indirect/src/scene_model.rs#L96)）把 provider、纹理、pass、材质、形状、节点、相机等包装进 `RenderArray`，用 `RenderMethod::TraditionalDraw(models.draw_command())` 提交——命令类型（MIDC 或降级后的 Indirect）由 provider 决定。
- host-driven 路径（`using_host_driven_indirect_draw`，即 CPU 每帧遍历但走间接提交）：[host_driven.rs:19](../../scene/rendering/gpu-indirect/src/host_driven.rs#L19) 用 `draw_command_host_access` 现场为每个实体生成 host 命令，再经 `downgrade_multi_indirect_draw_count_host_driven` 打包进同一套 helper 结构。

## 实现示例

[draw_cmd.rs:4](../../scene/rendering/gpu-indirect/src/shape/attribute/draw_cmd.rs#L4)（attribute mesh）是"一个数据源、两侧表达"的范本：同一份 `AttributeMeshIndirectDrawCreator` 同时实现 indexed 与 none-indexed 两套 builder，host 与 device 两侧共享同一批绑定（网格元数据表 + sm→mesh 映射）。其中 mesh 未分配时计数为零的命令正是空 drawcall 约定的落实点；u16 索引展开与 `used_in_midc_downgrade` 对基址单位的影响等完整实现细节见 [attribute-mesh-indirect-render-guide.md](attribute-mesh-indirect-render-guide.md)。

另有两类"包装式"实现，演示 builder 的两种组合方式：

- `AttributeLODMeshIndirectDrawCreator`（[attribute-mesh-lod/src/draw_cmd.rs:25](../../scene/rendering/attribute-mesh-lod/src/draw_cmd.rs#L25)）：包装内部网格 creator，`generate_draw_command` 里按相机投影、世界包围盒距离、节点缩放与屏幕空间误差阈值从粗到细选级（[draw_cmd.rs:196-215](../../scene/rendering/attribute-mesh-lod/src/draw_cmd.rs#L196)），host 侧只画根层级；接入方式见 attribute-mesh-indirect-render-guide 的「用户视角」。
- `InstanceDrawCommandBuilder`（[transform-instanced-model/src/indirect_draw/draw_cmd.rs:27](../../extension/transform-instanced-model/src/indirect_draw/draw_cmd.rs#L27)）：演示"生成器包装生成器"，`instance_meta` 提供实例数，内层生成源网格的命令后把 `vertex_count` 乘以实例数——这正是降级路径"实例数必须烘焙进 vertex_count"约束（见上文「MIDC 降级机制」）的落实点。

## 使用模板

### 模板一：接入一个新的间接绘制实现

实现 `NoneIndexedDrawCommandBuilder`（或 `Indexed` 对应物）三步：

- 实现 `draw_command_host_access`：host 侧直接构造 `DrawCommand`。
- 实现 `build_invocation`：把网格数据源绑进 shader，返回 invocation。
- 实现 `generate_draw_command`：按 draw_id（sm id）生成参数，`base_instance` 写 `draw_id`，未分配数据返回零计数命令。

以 [wide-styled-points/src/indirect_draw.rs:374](../../extension/wide-styled-points/src/indirect_draw.rs#L374) 的结构为例：

```rust
#[derive(Clone)]
pub struct WidePointsDrawCreator { /* 网格数据源 buffer ... */ }

impl NoneIndexedDrawCommandBuilder for WidePointsDrawCreator {
  fn draw_command_host_access(&self, id: EntityHandle<SceneModelEntity>) -> Option<DrawCommand> {
    // 读宿主侧数据, 构造 DrawCommand::Array
    Some(DrawCommand::Array { vertices: 0..count, instances: 0..1 })
  }
  fn build_invocation(&self, cx: &mut ShaderComputePipelineBuilder)
    -> Box<dyn NoneIndexedDrawCommandBuilderInvocation> {
    let data = cx.bind_by(&self.data);
    Box::new(DrawCmdBuilderInvocation { data })
  }
  fn bind(&self, builder: &mut BindingBuilder) { builder.bind(&self.data); }
}

impl NoneIndexedDrawCommandBuilderInvocation for DrawCmdBuilderInvocation {
  fn generate_draw_command(&self, draw_id: Node<u32>) -> Node<DrawIndirectArgsStorage> {
    // 未分配时计数为零 —— 空 drawcall 约定
    ENode::<DrawIndirectArgsStorage> {
      vertex_count: self.data.index(draw_id).load(),
      instance_count: val(1),
      base_vertex: val(0),
      base_instance: draw_id,
    }.construct()
  }
}
```

在渲染器的 `DrawCommandBuilderCreator::make_draw_command_builder` 里按需选分支，再让 `use_create_or_update_indirect_draw_providers` 调用 `use_and_create_default_indirect_draw_provider`（一行接入整条流水线，如 [wide-line/src/indirect_draw.rs:188](../../extension/wide-line/src/indirect_draw.rs#L188)）。降级开关按 `require_midc_downgrade` 传入，provider 创建时自动包 `MIDCDowngradeBatch`。

### 模板二：包装已有 builder

继承或组合现有 builder（LOD、实例化、单元网格均是此模式）：新 builder 持有内部 builder，`build_invocation` 里对内部 invocation 的结果做二次变换；`bind` 时先绑自己的资源再透传 `internal.bind(builder)`。注意 `ShaderHashProvider` 要合并内部哈希，否则管线缓存键缺项。

### 模板三：降级模式下保持顶点语义

顶点侧只允许通过注册的语义读 id（`VertexInstanceIndex` / `LogicalRenderEntityId`），不要直接假设 `VertexIndex` 的语义——降级路径下这两者都会由 helper 恢复。索引数据必须能走存储 buffer 读取（`MidcDowngradeWrapperForIndirectMeshSystem` 的约定），mesh 实现通过 `get_index_storage_buffer` 提供。

## 延伸阅读

- GPU 绘制列表与子列表范围：[shader/draw-list/src/lib.rs:9](../../shader/draw-list/src/lib.rs#L9)、[shader/draw-list/src/multi_range.rs](../../shader/draw-list/src/multi_range.rs)
- 并行计算组件抽象（ComputeComponent / DeviceInvocation）：[shader/parallel-compute/src/abstract_component.rs](../../shader/parallel-compute/src/abstract_component.rs)、[abstract_invocation.rs](../../shader/parallel-compute/src/abstract_invocation.rs)
- 段前缀和与 Kogge-Stone 扫描：[shader/parallel-compute/src/lib.rs:398](../../shader/parallel-compute/src/lib.rs#L398)、[shader/draw-list/src/stream_compact](../../shader/draw-list/src/stream_compact/mod.rs)（draw-list-guide 有流压缩总览）
- 间接绘制提交语义：[platform/graphics/webgpu/src/pass.rs:421](../../platform/graphics/webgpu/src/pass.rs#L421)
- 场景批提取与 PSO 分桶：[batch-extractor-guide.md](batch-extractor-guide.md)
