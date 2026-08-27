# Rendiation 两遍 GPU 遮挡剔除指南（scene/rendering/occlusion-culling）

本文梳理 [scene/rendering/occlusion-culling](../../scene/rendering/occlusion-culling/src/lib.rs) 的 GPU 遮挡剔除（occlusion culling，下文简称 OC）：如何用「上一帧可见性」把一个场景批拆成两遍——第一遍把上一帧可见的对象作为 occluder 绘制并生成深度金字塔（hierarchical z-buffer），第二遍用深度金字塔测试上一帧不可见对象、只绘制未被遮挡的——以及可见性状态如何经一个按场景模型实体 id 索引的 GPU 缓冲在帧间传递。读者应已了解 depth pyramid / hierarchical z-buffer 遮挡剔除的基本原理。

## 前置阅读

OC 建立在 GPU 绘制列表剔除、compute 管线与渲染帧组装之上，建议先了解：

| 文档 | 内容 |
| --- | --- |
| [draw-list-guide.md](draw-list-guide.md) | DeviceDrawList、`AbstractCullerProvider` 剔除抽象、流压缩（cull + prefix scan + scatter） |
| [skill-translation/shader-edsl-compute-zh.md](skill-translation/shader-edsl-compute-zh.md) | compute 管线构建、内置计算 ID、间接派发 |
| [skill-translation/shader-edsl-core-zh.md](skill-translation/shader-edsl-core-zh.md) | `Node<T>`、控制流、shader 内存操作 |
| [skill-translation/shader-edsl-binding-and-typed-container-zh.md](skill-translation/shader-edsl-binding-and-typed-container-zh.md) | `StorageBufferDataView`、`UniformBufferDataView` 与 `bind_by` 绑定 |
| [skill-translation/fundamental-gpu-component-model-zh.md](skill-translation/fundamental-gpu-component-model-zh.md) | `ShaderHashProvider` 与管线缓存（管线哈希的约定） |
| [skill-translation/frame-pass-assemble-zh.md](skill-translation/frame-pass-assemble-zh.md) | `FrameCtx`、pass 组装、颜色/深度附件的 load-store 操作 |
| [batch-extractor-guide.md](batch-extractor-guide.md) | 场景批提取与 occ-style-draw-control 分层绘制（OC 消费其产出的批次） |
| [skill-translation/scene-core-structure-zh.md](skill-translation/scene-core-structure-zh.md) | SceneModelEntity 实体 id 与场景模型数据 |

## 模式概览

经典的 hierarchical z-buffer 遮挡剔除是「先画出一批 occluder，再用它们的深度做查询」。rendiation 的 OC 把「谁是 occluder」交给上一帧的可见性来回答：

- **状态缓存**：`GPUTwoPassOcclusionCulling` 持有一个按场景模型实体 id 索引的 `[Bool]` 存储缓冲（`last_frame_visibility`），记录「该对象上一帧是否不可见」，以及一个跨帧复用的深度金字塔纹理缓存。
- **两遍渲染**：每帧把输入批拆成「上一帧可见」与「上一帧不可见」两个子批。第一遍绘制可见子批（occluder），从深度缓冲生成深度金字塔；第二遍用金字塔测试不可见子批，只绘制测试通过的。
- **帧间状态传递**：遮挡测试在 GPU 上执行的同时把结果写回状态缓冲，下一帧据此重新拆分——状态在帧间闭环，不需要 CPU 回读。
- **零初始化即安全默认**：状态缓冲存的是「不可见」标志（0 = 可见，1 = 不可见），以零初始化。未知实体默认「可见」，首帧全部进入 occluder 遍，状态从第二帧起逐帧收敛——因此不需要任何特殊初始化（见「帧间状态传递」一节）。
- **与 frustum 剔除互补**：[scene/rendering/frustum-culling](../../scene/rendering/frustum-culling/src/lib.rs) 的 `GPUFrustumCuller` 只做视锥测试，无法剔除「在视锥内但被其他物体挡住」的对象；OC 补上这一层。二者共用同一套 `AbstractCullerProvider` 抽象，viewer 中启用 OC 时关闭 frustum 剔除（见「用户视角」）。

## 核心概念

| 概念 | 定义 | 说明 |
| --- | --- | --- |
| `GPUTwoPassOcclusionCulling` | [scene/rendering/occlusion-culling/src/lib.rs:16](../../scene/rendering/occlusion-culling/src/lib.rs#L16) | OC 状态缓存 + `use_draw` 两遍流程入口 |
| `last_frame_visibility` | [lib.rs:18](../../scene/rendering/occlusion-culling/src/lib.rs#L18) | 每个 oc draw 的可见性状态：`StorageBufferDataView<[Bool]>`，按场景模型实体 id 索引 |
| `depth_pyramid_cache` | [lib.rs:20](../../scene/rendering/occlusion-culling/src/lib.rs#L20) | 深度金字塔纹理缓存（`Option<GPU2DTexture>`，尺寸变化才重建） |
| `GPUTwoPassOcclusionCullingResult` | [lib.rs:198](../../scene/rendering/occlusion-culling/src/lib.rs#L198) | 调试/采集用结果：`drawn_occluder` 与 `drawn_not_occluded` 两个批 |
| `OnlyLastFrameVisible` | [filter.rs:11](../../scene/rendering/occlusion-culling/src/filter.rs#L11) | 上一帧可见性过滤器（`AbstractCullerProvider`） |
| `OcclusionTester` | [occlusion_test.rs:55](../../scene/rendering/occlusion-culling/src/occlusion_test.rs#L55) | 深度金字塔遮挡测试：测试 AABB 并回写状态，同时本身是可复用的 culler |
| `AbstractCullerProvider` / `AbstractCullerInvocation` | [shader/draw-list/src/device_culling/mod.rs:9](../../shader/draw-list/src/device_culling/mod.rs#L9) | 剔除抽象：`cull(id) -> Node<bool>`，true == 应剔除（不可见） |
| `DrawUnitWorldBoundingProvider` | [scene/rendering/gpu-base/src/world_bounding.rs:3](../../scene/rendering/gpu-base/src/world_bounding.rs#L3) | id → 世界 AABB（`TargetWorldBounding`，HPT 高精度平移） |
| `compute_pot_enlarged_hierarchy_depth` | [shader/fast-down-sampling-2d/src/entry.rs:54](../../shader/fast-down-sampling-2d/src/entry.rs#L54) | depth → 深度金字塔生成（外部库，pot 放大 + 逐 mip 归约） |
| `GPUFrustumCuller` | [scene/rendering/frustum-culling/src/lib.rs:72](../../scene/rendering/frustum-culling/src/lib.rs#L72) | 视锥剔除器，与 OC 互补 |
| `ViewerOcclusionCulling` | [application/viewer-content/src/rendering/culling.rs:65](../../application/viewer-content/src/rendering/culling.rs#L65) | viewer 侧按相机持有 OC 状态并收集剔除结果 |

## 分层动机与数据流

先看完整的两遍数据流与帧间状态传递，再逐层展开：

```text
帧 N 开始:last_frame_visibility[] = 上一帧测试写入的状态(0=可见,1=不可见;初始全 0)

  batch(全部不透明场景模型,以实体 id 为索引)
  │
  ├─ filter「上帧可见」(cull = 状态值==1)
  │    └─ last_frame_visible_batch
  │         │
  │         │  第一遍 occluder pass:绘制该批(可带背景 preflight)
  │         ▼
  │       depth buffer
  │         │
  │         │  生成深度金字塔:copy 到 pot 尺寸的 R32Float 纹理,
  │         │  逐 mip 做 2x2 归约(normal 取 max / reverse 取 min)
  │         ▼
  │       depth_pyramid(缓存)
  │         │
  │         │  compute:对 visible_batch 每个对象测试 AABB vs 金字塔,
  │         │  结果写回 last_frame_visibility[id](→ 下一帧),同时返回 tester
  │         ▼
  │       OcclusionTester culler
  │
  └─ filter.not「上帧不可见」(含 id 越界的对象)
       └─ last_frame_invisible_batch
            │
            │  第二遍剔除:pre_culler.shortcut_or(occlusion_tester)
            │  (tester 在此也顺带回写状态,见「帧间状态传递」)
            ▼
         second_pass_batch
            │
            │  第二遍 subject pass:绘制,深度全部改 load(不清第一遍)
            ▼
         帧 N 完成:last_frame_visibility[] 已更新,帧 N+1 直接复用
```

分层动机：

- **时间复用**：被遮挡物体的集合帧间变化通常不大。「上一帧可见 → 本帧当作 occluder 直接画，不再因测试结果而剔除」；反过来，上一帧可见的对象必须被测试（相机移动后它们可能变成遮挡者身后），这就是第一遍后那段测试 compute 的职责。注意两批对象每帧都会经过金字塔测试——可见批在测试 compute 里、不可见批在第二遍剔除里——差别只在「测试结果是否用于本帧的绘制剔除」：可见批的测试只更新下一帧状态，不可见批的测试直接决定本帧画不画。
- **两遍各司其职**：第一遍产出深度（occluder 集），第二遍消费深度（subject 集）。两遍写同一个深度目标、第二遍不清深度，occluder 与通过测试的 subject 合起来构成完整帧。
- **状态存「不可见」而非「可见」**：零初始化使未知对象默认可见——首帧全部进 occluder 遍、全部被绘制，正确性从头成立，且无需向缓冲写入任何初始化数据（若反过来存可见标志，零初始化会把首帧变成「全部不可见」）。
- **金字塔缓存**：深度金字塔尺寸只随视口变化，`depth_pyramid_cache` 按尺寸/mip 数复用纹理，避免逐帧重建。
- **相机相对渲染（HPT）**：世界 AABB 与相机位置都以「高精度平移」（f64 拆成两个 f32，见 [shader/api/src/graphics/high_precision_translation.rs:38](../../shader/api/src/graphics/high_precision_translation.rs#L38)）存储，测试时先做 `hpt_sub_hpt`（[同文件:90](../../shader/api/src/graphics/high_precision_translation.rs#L90)）把包围盒转为相机相对坐标，再乘不含平移的 view-projection（`CameraGPUTransform::view_projection_without_translation`，[scene/rendering/gpu-base/src/camera.rs:148](../../scene/rendering/gpu-base/src/camera.rs#L148)），保证大世界坐标下的精度。

### 两遍的职责划分

「两遍」的分工围绕一个时间假设：**上一帧可见的对象大概率本帧仍可见，而不可见的对象需要被重新验证**。三件事各自落在两遍之间：

| 阶段 | 处理的批次 | 做什么 | 产出 |
| --- | --- | --- | --- |
| 第一遍（occluder pass） | 上帧可见批 | 绘制该批（含背景 preflight），写深度 | 深度缓冲 |
| 金字塔生成（compute/render 混合） | — | 深度拷贝到 pot 尺寸 R32Float 纹理，逐 mip 2x2 归约 | 深度金字塔（缓存） |
| 遮挡测试（compute） | 上帧可见批 | 每个对象测试 AABB vs 金字塔，回写状态 | 下一帧状态 + `OcclusionTester` culler |
| 第二遍（subject pass） | 上帧不可见批 | 用 `pre_culler.shortcut_or(tester)` 剔除后绘制，深度 load 不清第一遍 | 完整帧的其余对象 |

要点是**上帧可见批不再做金字塔剔除**（直接信任并作为 occluder；对它们跑的金字塔测试只用于更新下一帧状态），而**上帧不可见批全部接受金字塔剔除**（它们可能只是上一帧恰好被挡，本帧已露出）——剔除开销与「真正需要验证的对象数」成正比，这是时间复用的核心收益。两遍写同一份深度目标且第二遍不清深度，occluder 与通过测试的 subject 拼成完整一帧。

## 状态缓存与构造

[lib.rs:16](../../scene/rendering/occlusion-culling/src/lib.rs#L16) 的 `GPUTwoPassOcclusionCulling` 只有两个字段：状态缓冲与金字塔缓存，全部是 GPU 资源，构造极轻：

```rust
pub fn new(max_scene_model_id: usize, gpu: &GPU) -> Self {
  let init = ZeroedArrayByArrayLength(max_scene_model_id);
  let last_frame_visibility = create_gpu_read_write_storage(init, gpu, "last_frame_visibility");
  ...
}
```

- `max_scene_model_id` 是**场景模型实体 id 的最大值**（不是批的大小）：它决定状态缓冲长度，因为缓冲以实体 id 为下标。`ZeroedArrayByArrayLength`（[platform/graphics/webgpu/src/resource/buffer/storage.rs:220](../../platform/graphics/webgpu/src/resource/buffer/storage.rs#L220)）创建按字节数零初始化的缓冲。
- 参数注释强调缓冲**不能**按输入批大小动态伸缩：输入批表示「当前批最多可能出现的模型数」，而缓冲需要覆盖整个场景的 id 空间。
- 越界的 id（实体 id ≥ 缓冲长度）在过滤器中恒被当作「不可见」处理（见下节），因此始终走第二遍测试——OC 对它们不生效，但正确性有保证（[lib.rs:26](../../scene/rendering/occlusion-culling/src/lib.rs#L26) 的注释语义）。
- `Bool` 是 u32 承载的 shader 布尔（[shader/api/src/api_core/expr/primitive.rs:507](../../shader/api/src/api_core/expr/primitive.rs#L507)）：`Node<Bool>::into_bool()` 做 `!= 0` 比较，`Node<bool>::into_big_bool()` 把布尔映射为 1/0 的 u32（[同文件:529](../../shader/api/src/api_core/expr/primitive.rs#L529)）。状态缓冲里 1 = 上一帧不可见、0 = 可见。

## 批次拆分：filter.rs

`use_draw` 开头先用两个互补的过滤器把批一分为二（[lib.rs:72](../../scene/rendering/occlusion-culling/src/lib.rs#L72)）：

```rust
let last_frame_visible_batch = batch.use_culled_list_and_do_culling(cx, filter_last_frame_visible_object(last_frame_invisible));
let last_frame_invisible_batch = batch.use_culled_list_and_do_culling(cx, filter_last_frame_visible_object(last_frame_invisible).not());
```

- `use_culled_list_and_do_culling`（[scene/rendering/gpu-base/src/batch.rs:37](../../scene/rendering/gpu-base/src/batch.rs#L37)，内部走 [shader/draw-list/src/stream_compact/mod.rs:10](../../shader/draw-list/src/stream_compact/mod.rs#L10) 的 cull + 流压缩）对每个 id 求 `cull(id)`，把存活者压紧成新列表——过滤本身就是一个标准的 GPU 剔除。
- `OnlyLastFrameVisible::cull`（[filter.rs:39](../../scene/rendering/occlusion-culling/src/filter.rs#L39)）：id 在缓冲范围内时直接返回缓冲值（1 = 不可见 = 剔除）；范围外返回 true（恒不可见）。`.not()` 是 `AbstractCullerProviderExt` 的组合子（[shader/draw-list/src/device_culling/mod.rs:45](../../shader/draw-list/src/device_culling/mod.rs#L45)），翻转两个子批。

两次流压缩都有 GPU 代价（注释 `todo, this should be optimized`），但换来的是两个批天然互斥且各自压紧。

## 第一遍：occluder pass

[lib.rs:86](../../scene/rendering/occlusion-culling/src/lib.rs#L86)：

- 以 `last_frame_visible_batch` 为 occluder 集。`generate_culling_result` 为真时，先额外施加调用方传入的 `pre_culler`（例如 frustum 剔除）再画——这个选项用于「结果需要对外汇报」的场景，见「结果消费」。
- pass 命名为 `occlusion-culling-first-pass`，绘制前先执行 `preflight_content`（如背景绘制，见 [lib.rs:43](../../scene/rendering/occlusion-culling/src/lib.rs#L43) 的说明：用回调支持不另开 pass 画背景）。
- 该 pass 使用调用方传入的 `target` 原样（clear/load 行为由调用方决定），并**写深度**——这份深度就是后续金字塔的输入。

## 深度金字塔生成

第一遍结束后立即从该深度缓冲生成金字塔（[lib.rs:109](../../scene/rendering/occlusion-culling/src/lib.rs#L109)）：

- 目标纹理用 `next_pot_sizer`（[shader/fast-down-sampling-2d/src/entry.rs:47](../../shader/fast-down-sampling-2d/src/entry.rs#L47)）把尺寸放大到 2 的幂，mip 数为完整链（`MipLevelCount::BySize`，[platform/graphics/webgpu/src/resource/texture/d2.rs:212](../../platform/graphics/webgpu/src/resource/texture/d2.rs#L212)）。pot 尺寸保证每一级 mip 恰好是上一级的一半，且「mip 级纹理的大小 == 像素块大小」的映射成立。
- 格式为 `R32Float`：注释说明 depth32float 不能用作 storage texture 绑定（[lib.rs:132](../../scene/rendering/occlusion-culling/src/lib.rs#L132)），所以要先拷贝成单通道 float。
- 缓存：尺寸或 mip 数变化才重建，否则复用 `depth_pyramid_cache`（[lib.rs:117](../../scene/rendering/occlusion-culling/src/lib.rs#L117)，注释标注 `todo, make it transient`）。
- 生成算法 `compute_pot_enlarged_hierarchy_depth`（[entry.rs:54](../../shader/fast-down-sampling-2d/src/entry.rs#L54)）：单采样深度时先用一个全屏 quad 把深度拷贝进 mip0（[entry.rs:105](../../shader/fast-down-sampling-2d/src/entry.rs#L105)），再对每级 mip 做 2x2 归约（`compute_hierarchy_depth_from_depth_texture`，[entry.rs:28](../../shader/fast-down-sampling-2d/src/entry.rs#L28)）；MSAA 深度则直接读 4 个采样点归约。归约器 `depth_reducer`（[entry.rs:20](../../shader/fast-down-sampling-2d/src/entry.rs#L20)）：普通深度（reverse_depth=false）取 **max**（每个金字塔纹素 = 覆盖区域的最深/最远深度），reverse 深度取 **min**。这正是测试判定所需的最坏情况语义。

## 遮挡测试 compute：occlusion_test.rs

### 测试并回写状态

`test_and_update_last_frame_visibility_for_last_frame_visible_batch_and_return_culler`（[occlusion_test.rs:3](../../scene/rendering/occlusion-culling/src/occlusion_test.rs#L3)）构建一个 compute 管线并派发，对 `last_frame_visible_batch` 的每个对象执行遮挡测试，同时把结果写回状态缓冲：

- 管线按 `ShaderHashProvider` 惯例哈希（draw list + tester，[occlusion_test.rs:23](../../scene/rendering/occlusion-culling/src/occlusion_test.rs#L23)），由 `get_or_cache_create_compute_pipeline_by` 走管线缓存。
- 用 `last_frame_visible_list.invocation_logic` 拿到 `(实体 id, valid)`，`if_by(valid, ...)` 里执行 `culler.cull(id)`（[occlusion_test.rs:32](../../scene/rendering/occlusion-culling/src/occlusion_test.rs#L32)）。
- 派发走间接派发：`compute_work_size` 给出 indirect dispatch buffer，`dispatch_workgroups_indirect_by_buffer_resource_view` 按批的实际存活数派发（[occlusion_test.rs:41](../../scene/rendering/occlusion-culling/src/occlusion_test.rs#L41)）。
- 测试副作用：`OcclusionTesterInvocation::cull` 在 id 范围内时把 `is_occluded` 写进 `last_frame_invisible[id]`（[occlusion_test.rs:106](../../scene/rendering/occlusion-culling/src/occlusion_test.rs#L106)），并返回 `is_occluded` 作为剔除判定——「测试」与「状态更新」是同一个 shader 的两面。

### is_occluded：经典 hierarchical z-buffer 测试

[occlusion_test.rs:120](../../scene/rendering/occlusion-culling/src/occlusion_test.rs#L120) 是整个算法的核心，返回 true == 被遮挡：

- **空盒约定**：`min.x > max.x`（取 HPT 的 f1 分量判断）表示「不需要剔除的空盒」，跳过测试并判为可见（[occlusion_test.rs:124](../../scene/rendering/occlusion-culling/src/occlusion_test.rs#L124)）。世界包围盒查询返回 `None`（视图相关/动态对象）时写空盒（[world_bounding.rs:33](../../scene/rendering/gpu-base/src/world_bounding.rs#L33)），frustum culler 用同一约定（[frustum-culling/src/lib.rs:139](../../scene/rendering/frustum-culling/src/lib.rs#L139)）。
- **8 角点投影**：AABB 相对相机（`hpt_sub_hpt(box.min, camera_world)`），用 `view_projection_without_translation` 变换，透视除法到 NDC，clamp 到 [-1,1]³ 后转 UV。8 个角点用 `val(8).into_shader_iter()` + `switch_by` 按位组合生成（[occlusion_test.rs:145](../../scene/rendering/occlusion-culling/src/occlusion_test.rs#L145)）。
- **屏幕空间包围盒与最近深度**：迭代中累积 `min_xy/max_xy`（投影后的 UV 包围盒）与 `shallowest_bbox_depth`——normal 深度取各角点 z 的最小值（盒子的最近点），reverse 深度取最大值（[occlusion_test.rs:195](../../scene/rendering/occlusion-culling/src/occlusion_test.rs#L195)）。
- **mip 选择**：`box_size = (max_xy - min_xy) * mip0 尺寸`，`mip = ceil(log2(max(w, h)))`，clamp 到 `[0, mip 数-1]`（[occlusion_test.rs:205](../../scene/rendering/occlusion-culling/src/occlusion_test.rs#L205)）——选出「一个纹素约等于一个像素块」的那一级，屏幕投影越小选级越深，测试越便宜。
- **2x2 采样与判定**：在该 mip 上取包围盒左上角邻域的 2x2 纹素，normal 深度取四个的 max（该足迹内最深/最远，`d_0.max(...)`），reverse 取 min（[occlusion_test.rs:225](../../scene/rendering/occlusion-culling/src/occlusion_test.rs#L225)）。判定：盒子的最近点深度 **大于**（normal）/ **小于**（reverse）足迹最深深度 ⇒ 整个盒子被挡住 ⇒ occluded。取「最坏情况深度」与「盒子最近点」比较是保守方向，杜绝误剔除。

## 第二遍：subject pass

[lib.rs:164](../../scene/rendering/occlusion-culling/src/lib.rs#L164)：

```rust
let second_pass_culler = pre_culler.shortcut_or(occlusion_culler);
let second_pass_batch = last_frame_invisible_batch.use_culled_list_and_do_culling(cx, second_pass_culler);
```

- `shortcut_or`（[shader/draw-list/src/device_culling/operator.rs:52](../../shader/draw-list/src/device_culling/operator.rs#L52)）是短路 OR：`pre_culler` 已判定剔除的对象不再跑遮挡测试；被 `pre_culler` 放行的才进入 `OcclusionTester`。viewer 里启用 OC 时 `pre_culler` 为 `NoopCuller`（什么都不剔除，[operator.rs:4](../../shader/draw-list/src/device_culling/operator.rs#L4)），所有不可见批对象都要经过金字塔测试。
- **状态回写的第二处**：`OcclusionTester::cull` 的写副作用在第二遍剔除中同样发生——`shortcut_or` 在短路另一侧放行时执行 tester，于是「上一帧不可见、本帧测试通过」的对象状态被置 0，下一帧升格为 occluder。viewer 里 `pre_culler` 为 `NoopCuller`，因此批里的全部对象每帧都被测试并回写；若调用方传入真实 `pre_culler`，被它剔除的对象不会跑遮挡测试、状态保持原值（仍为不可见，与剔除结果一致）。两处写入合起来让状态每帧完整更新。
- 第二遍 pass 前调用 `make_all_channel_and_depth_into_load_op()`（[lib.rs:177](../../scene/rendering/occlusion-culling/src/lib.rs#L177)），保证**不清除第一遍已画的深度**——两遍深度合成同一帧。
- `generate_culling_result` 为真时，把两个批打包成 `GPUTwoPassOcclusionCullingResult` 返回（[lib.rs:184](../../scene/rendering/occlusion-culling/src/lib.rs#L184)）。

## 帧间状态传递（设计要点）

把状态流串起来：

- 缓冲存的是**「不可见」标志**（1 = 上一帧不可见），而不是「可见」标志。
- 缓冲以**零初始化**，因此新实体（或从未被测试过的 id）默认「可见」——首帧所有对象进入 occluder 遍被绘制并测试，从第二帧起每个对象的标志反映上一帧的真实测试结果，逐帧收敛。
- 这样做的收益正是注释（[lib.rs:17](../../scene/rendering/occlusion-culling/src/lib.rs#L17)）所说的「不需要特殊缓冲初始化」：若存可见标志，就必须把缓冲初始化为全 1 才能让首帧安全；而存不可见标志时零初始化的默认态恰好是安全态。id 越界的对象由过滤器兜底为「不可见」，永远走第二遍测试，同样保证正确性。
- 状态完全在 GPU 上读写：第一遍后的测试 compute 写可见批的状态，第二遍剔除 compute 写不可见批的状态（经 `pre_culler` 放行者），CPU 侧从不回读——这是「两遍 + 状态缓冲」得以零 CPU 开销逐帧闭环的关键。

## 结果消费与调试

`GPUTwoPassOcclusionCullingResult` 的消费方（viewer 侧）：

- **debug 相机**：`use_draw_with_oc_maybe_enabled` 检查 `viewport.debug_camera_for_view_related`，若有则用前一相机缓存的 `culling_results` 把 occluder 批与未遮挡批画进调试视角，直接可视化两遍划分（[culling.rs:165](../../application/viewer-content/src/rendering/culling.rs#L165)）。
- **批采集**：帧末 `feedback_culling_result`（[culling.rs:223](../../application/viewer-content/src/rendering/culling.rs#L223)）把每个相机的两个批交给 `RenderBatchCollector`（[scene/rendering/scheduler/src/lib.rs:11](../../scene/rendering/scheduler/src/lib.rs#L11)，如拾取等需要「实际被画了哪些对象」的子系统）。frame 组装处用 `batch_collector.will_collecting()` 决定是否保留结果（`set_should_keep_oc_cull_result`，[frame_all.rs:633](../../application/viewer-content/src/rendering/frame_all.rs#L633)，对应 `generate_culling_result`），帧末统一 `feedback_culling_result`（[frame_all.rs:690](../../application/viewer-content/src/rendering/frame_all.rs#L690)）。

## 用户视角：构造与逐帧使用

- **配置**：`ViewerCullingConfig` 两个开关——`enable_indirect_occlusion_culling`（[init_config.rs:14](../../application/viewer-content/src/init_config.rs#L14)）与 `occlusion_culling_max_scene_model_count`（[init_config.rs:43](../../application/viewer-content/src/init_config.rs#L43)，默认 `u16::MAX`）；后者就是 `GPUTwoPassOcclusionCulling::new` 的 `max_scene_model_id`，应按场景模型 id 空间保守设置。
- **构造**：`use_viewer_culling` 仅当配置开启且为 indirect 路径时创建 OC 状态（[culling.rs:15](../../application/viewer-content/src/rendering/culling.rs#L15)）：先把视口列表经 `per_camera_per_viewport` 按相机聚合（多个视口共享同一相机时合并为一项，调试相机与主相机分开），再对每个相机在一个 `cx.keyed_scope(&cv.camera)` 里用 `use_sharable_plain_state` 创建一个 `GPUTwoPassOcclusionCulling`，存在 `FastHashMap<相机, Arc<RwLock<GPUTwoPassOcclusionCulling>>>` 里——**每个相机一份状态**，不同相机的可见性互不干扰。
- **逐帧更新**：光照 pass 的 opaque 分支调用 `use_draw_with_oc_maybe_enabled`（[culling.rs:141](../../application/viewer-content/src/rendering/culling.rs#L141)，调用点 [lighting/light_pass/mod.rs:97](../../application/viewer-content/src/rendering/lighting/light_pass/mod.rs#L97)）：OC 开启时跳过 frustum 剔除（两者都做同样的事，[culling.rs:158](../../application/viewer-content/src/rendering/culling.rs#L158)），随后 `oc_state.write().use_draw(...)` 完成两遍（[culling.rs:189](../../application/viewer-content/src/rendering/culling.rs#L189)），返回的 pass 继续后续透明/后处理管线。状态缓冲与金字塔缓存都在 `GPUTwoPassOcclusionCulling` 内部，逐帧更新就是反复调用 `use_draw`。
- **查询**：每帧的剔除结果按相机存入 `culling_results`（仅当 `always_keep_cull_result`（调试）或 `should_keep_cull_result`（采集器激活）时生成，[culling.rs:205](../../application/viewer-content/src/rendering/culling.rs#L205)），供 debug 相机与批采集消费。

## 使用模板

### 模板一：在 viewer 中装配 OC

viewer 的装配分两步。配置来自 `ViewerCullingConfig`（[frame_all.rs:31](../../application/viewer-content/src/rendering/frame_all.rs#L31)），默认关闭，egui 面板里可勾选 `enable_indirect_occlusion_culling`（[rendering/egui.rs:118](../../application/viewer-content/src/rendering/egui.rs#L118)）：

```rust
// application/viewer-content/src/init_config.rs
pub enable_indirect_occlusion_culling: bool,          // L14
pub occlusion_culling_max_scene_model_count: u32,     // L43,默认 u16::MAX
```

状态构造在 `use_viewer_culling`（[culling.rs:15](../../application/viewer-content/src/rendering/culling.rs#L15)）：仅当配置开启且是 indirect 渲染路径时，视口列表先经 `per_camera_per_viewport` 按相机聚合，再对每个相机在一个 keyed scope 里创建共享状态：

```rust
let cache = cx.keyed_scope(&cv.camera, |cx| {
  cx.use_sharable_plain_state(|| {
    GPUTwoPassOcclusionCulling::new(
      config.occlusion_culling_max_scene_model_count as usize,
      cx.gpu,
    )
  })
});
```

### 模板二：自定义渲染器中逐帧调用 use_draw

`use_draw` 的参数就是两遍流程的全部输入（调用点在 [culling.rs:190](../../application/viewer-content/src/rendering/culling.rs#L190)）：

```rust
let (pass, cull_result) = oc_state.write().use_draw(
  ctx,
  &reorderable_batch.get_device_batch().unwrap(), // 输入:全部不透明对象的 device 批
  None,                      // pre_culler:OC 开启时不再叠加 frustum 剔除
  pass_base,                 // RenderPassDescription(颜色/深度附件与初始 clear 行为)
  preflight_content,         // 背景绘制回调,在 occluder pass 内先画
  renderer.scene,            // SceneRenderer:把批转成 pass content
  camera_gpu,
  scene_pass_dispatcher,     // RenderComponent(材质/状态写入等)
  bounding_provider.clone(), // DrawUnitWorldBoundingProvider:id → 世界 AABB
  renderer.reversed_depth,   // 反向深度
  generate_culling_result,   // 需要结果(调试/采集)时置 true
);
```

返回的 `ActiveRenderPass` 继续后续管线（透明绘制、后处理），与 `use_draw` 之前的状态无关——它只是「场景 opaque 部分画完之后的 pass」。状态缓冲与金字塔缓存在 `GPUTwoPassOcclusionCulling` 内部自持，逐帧反复调用 `use_draw` 即完成更新。

### 模板三：查询剔除结果

结果查询有两条路径：

- `generate_culling_result = true` 时返回 `GPUTwoPassOcclusionCullingResult { drawn_occluder, drawn_not_occluded }`（[lib.rs:198](../../scene/rendering/occlusion-culling/src/lib.rs#L198)）。viewer 把它按相机存进 `ViewerOcclusionCulling::culling_results`（[culling.rs:205](../../application/viewer-content/src/rendering/culling.rs#L205)），debug 相机视口据此重画两个批（[culling.rs:165](../../application/viewer-content/src/rendering/culling.rs#L165)）。
- 帧末 `feedback_culling_result` 把每个相机的两个批交给 `RenderBatchCollector`（[culling.rs:223](../../application/viewer-content/src/rendering/culling.rs#L223)），供拾取等需要「实际被画的对象集合」的子系统复用；frame 组装处用 `will_collecting()` 决定本帧是否保留结果（[frame_all.rs:633](../../application/viewer-content/src/rendering/frame_all.rs#L633)）。

## 延伸阅读

- GPU 剔除抽象与流压缩：[shader/draw-list/src/device_culling/mod.rs](../../shader/draw-list/src/device_culling/mod.rs)、[shader/draw-list/src/stream_compact/mod.rs](../../shader/draw-list/src/stream_compact/mod.rs)
- 通用 2D 快速降采样机制（本 crate 只用到入口函数）：[shader/fast-down-sampling-2d/src/lib.rs](../../shader/fast-down-sampling-2d/src/lib.rs)
- 高精度平移与相机相对渲染：[shader/api/src/graphics/high_precision_translation.rs](../../shader/api/src/graphics/high_precision_translation.rs)
- 世界包围盒查询与空盒约定：[application/viewer-content/src/bounding.rs](../../application/viewer-content/src/bounding.rs)、[scene/rendering/gpu-base/src/world_bounding.rs](../../scene/rendering/gpu-base/src/world_bounding.rs)
- 视锥剔除与 OC 的互补关系：[scene/rendering/frustum-culling/src/lib.rs](../../scene/rendering/frustum-culling/src/lib.rs)
