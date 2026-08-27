# Rendiation GPU 并行原语指南（shader/parallel-compute）

本文梳理 [shader/parallel-compute](../../shader/parallel-compute/src/lib.rs) 的 GPU 并行算法原语：流压缩（stream compaction）、段前缀和/前缀和（segmented prefix scan / Kogge-Stone 扫描）、radix sort，以及支撑它们的组件模型（惰性组合、物化、直接/间接派发）。渲染器里所有"在 GPU 上压缩/扫描/排序一个数组"的需求——draw-list 的剔除流压缩、MIDC 降级管线的段前缀和、遮挡剔除的批次拆分——最终都落在这一层原语上。

## 前置阅读

本 crate 用 shader EDSL 写 compute shader，先了解 EDSL 基础与 GPU 单元测试模式：

| 文档 | 内容 |
| --- | --- |
| [skill-translation/shader-edsl-compute-zh.md](skill-translation/shader-edsl-compute-zh.md) | 计算管线构建、工作组共享内存、`workgroup_barrier`、GPU 单元测试（`#[pollster::test]` + 回读） |
| [skill-translation/shader-edsl-core-zh.md](skill-translation/shader-edsl-core-zh.md) | `Node<T>` / `ShaderPtrOf`、`if_by` / `loop_by`、`into_shader_iter` 等语言基础 |
| [skill-translation/shader-edsl-binding-and-typed-container-zh.md](skill-translation/shader-edsl-binding-and-typed-container-zh.md) | `StorageBufferDataView` / `StorageBufferReadonlyDataView` 与绑定 |
| [draw-list-guide.md](draw-list-guide.md) | `DeviceDrawList` 与 GPU 剔除流程（流压缩的下游消费方） |
| [indirect-draw-command-guide.md](indirect-draw-command-guide.md) | 间接绘制命令与 MIDC 降级机制（段前缀和的另一个下游） |

## 模式概览

渲染流程里反复出现"对一串数据做 GPU 并行处理"的需求：剔除后把存活模型 id 压紧、为每个子列表计算绘制数量前缀和、按位稳定排序。本 crate 把这些算法统一成一种可组合的表述：

- **组件模型**：`ComputeComponent<T>` 描述"每个线程做什么"，用 `map` / `zip` / `offset_access` / `stride_access_result` 惰性组合，任何环节都可以物化（materialize）成 storage buffer。
- **monoid 抽象**：`DeviceMonoidLogic { identity, combine }` 把"加、取最大、逻辑或"等结合运算注入扫描与归约，算法本身与元素类型、运算完全解耦。
- **工作组内扫描**：Kogge-Stone 算法，共享内存 + 双屏障的 log n 步 inclusive 扫描，一步到位。
- **段前缀和**：两阶段全局扫描（块内扫描 → 块尾扫描 → 合并），把可扫描规模从"一个工作组"（设备上限，通常 256）扩展到 first_stage × second_stage。
- **流压缩**：mask → 段前缀和 → 从 inclusive 结果推导排他位置 → 每个线程把自己写到目标位置（scatter），结果长度由前缀和尾元素给出。
- **动态派发**：宿主侧不知道规模（如剔除后的数量）时，先跑一个 size pass 算出派发参数，再间接派发。

## 核心概念

| 概念 | 定义 | 说明 |
| --- | --- | --- |
| `ComputeComponent<T>` | [abstract_component.rs:3](../../shader/parallel-compute/src/abstract_component.rs#L3) | 惰性 compute 组件：`work_size` / `result_size` / `requested_workgroup_size` / `build_shader` / `bind_input` |
| `DeviceInvocation<T>` | [abstract_invocation.rs:5](../../shader/parallel-compute/src/abstract_invocation.rs#L5) | 单线程逻辑：`invocation_logic(id) → (值, valid)`，`invocation_size()` 定义线程范围 |
| `ComputeComponentIO<T>` | [abstract_component.rs:144](../../shader/parallel-compute/src/abstract_component.rs#L144) | 可物化的组件：`use_materialize_storage_buffer` 落地成 `DeviceMaterializeResult` |
| `DeviceMaterializeResult<T>` | [io.rs:70](../../shader/parallel-compute/src/io.rs#L70) | 已物化结果：只读 buffer + 可选 size（`Vec4<u32>`，x 为有效长度） |
| `DeviceMonoidLogic` | [prefix_scan.rs:10](../../shader/parallel-compute/src/prefix_scan.rs#L10) | monoid 注入点：`identity()` + `combine(a, b)`（结合律） |
| `AdditionMonoid<T>` | [prefix_scan.rs:20](../../shader/parallel-compute/src/prefix_scan.rs#L20) | 加法 monoid，最常用 |
| `WorkGroupPrefixScanKoggeStoneCompute` | [prefix_scan.rs:39](../../shader/parallel-compute/src/prefix_scan.rs#L39) | 工作组内 inclusive Kogge-Stone 扫描 |
| `use_segmented_prefix_scan_kogge_stone` | [lib.rs:398](../../shader/parallel-compute/src/lib.rs#L398) | 两阶段全局 inclusive 扫描，规模上限 first_stage × second_stage |
| `make_global_scan_exclusive` | [lib.rs:432](../../shader/parallel-compute/src/lib.rs#L432) | inclusive → exclusive（整体右移一位，边界填 identity） |
| `use_stream_compaction` | [stream_compaction.rs:3](../../shader/parallel-compute/src/stream_compaction.rs#L3) | 流压缩：mask → 段前缀和 → 推导排他位置 → scatter |
| `shuffle_move` | [lib.rs:224](../../shader/parallel-compute/src/lib.rs#L224) | scatter：每个线程把数据写到指定位置 |
| `offset_access` / `stride_access_result` | [access_behavior.rs:59](../../shader/parallel-compute/src/access_behavior.rs#L59)、[stride_read.rs:24](../../shader/parallel-compute/src/stride_read.rs#L24) | 位置变换组合子：平移读取、按步长抽取/复制 |
| `gpu_cx!` / `run_test` | [ctx.rs:245](../../shader/parallel-compute/src/ctx.rs#L245)、[lib.rs:142](../../shader/parallel-compute/src/lib.rs#L142) | GPU 单元测试环境与断言 |

## 分层动机与数据流

先看完整数据流，再逐层展开：

```text
输入（host 数据 / 上一个 GPU 组件 / DeviceDrawList）
  └─ ComputeComponent 惰性组合（map / zip / offset_access / stride_read）
       └─ 落地时机由消费者决定：
            ├─ use_dispatch_compute：直接派发（work_size 已知）
            └─ use_materialize_storage_buffer：写入缓存 buffer，返回只读 view
                 └─ work_size 未知时：先跑 size pass → 间接派发
                      └─ DeviceMaterializeResult { buffer, size }
                           └─ 作为下一个组件的输入（扫描 → 压缩 → 排序 …）
```

分层动机：

- **惰性与物化解耦**。`map` / `zip` 等组合子只生成 shader 结构，不触发任何 dispatch；只有物化（或最终派发）时才真正生成管线与执行。中间步骤零拷贝——下游直接绑定上游的 buffer view。
- **规模与工作组解耦**。单个工作组最多扫描 `max_compute_invocations_per_workgroup` 个元素（WebGPU 上限通常 256），而渲染列表常有数万条目；两阶段扫描把上限提到 256² 且只有 3 次 dispatch。
- **类型与运算解耦**。扫描/归约/压缩对元素类型与结合运算完全泛型（`DeviceMonoidLogic`），新增一种"扫描语义"只需提供 identity + combine。
- **派发方式与算法解耦**。宿主侧不知道的规模（剔除后的存活数）由 `invocation_size()` 在 GPU 上实时算出，自动切换间接派发。

## 组合子语义：map / zip / offset_access / stride

所有算法都建立在少数几个惰性组合子上（`DeviceParallelComputeExt`，[lib.rs:55](../../shader/parallel-compute/src/lib.rs#L55)）：

| 组合子 | 位置 | 语义 |
| --- | --- | --- |
| `map(f)` | [lib.rs:80](../../shader/parallel-compute/src/lib.rs#L80) | 逐元素变换，可换元素类型（如 mask 转 1/0） |
| `zip(other)` | [lib.rs:121](../../shader/parallel-compute/src/lib.rs#L121) | 两个同规模组件逐元素配对 |
| `offset_access(offset, ob, size_expand)` | [lib.rs:342](../../shader/parallel-compute/src/lib.rs#L342) | 读"位置 + offset"的元素，越界按 `ClampBorder` / `Const` 处理，可延长结果 |
| `stride_reduce_result(stride)` | [lib.rs:61](../../shader/parallel-compute/src/lib.rs#L61) | 每 stride 个元素抽一个（抽取，尺寸除以 stride） |
| `stride_expand_result(stride)` | [lib.rs:66](../../shader/parallel-compute/src/lib.rs#L66) | 每个元素复制 stride 份（展开，尺寸乘以 stride） |

它们只是"包装上游的 `DeviceInvocation`"，不改动任何数据——组合树里每个节点都可以直接 `build_shader`，也可以在某处截断物化。`offset_access` 的越界行为由 `OutBoundsBehavior` 描述（[access_behavior.rs:20](../../shader/parallel-compute/src/access_behavior.rs#L20)）：`ClampBorder` 钳到边界元素，`Const(f)` 填常量（如扫描的 identity）。注意其实现会在 `target` 越界时调用 `source.start_point()` / `end_point()`（[access_behavior.rs:81](../../shader/parallel-compute/src/access_behavior.rs#L81)）——这两个方法（[abstract_invocation.rs:11](../../shader/parallel-compute/src/abstract_invocation.rs#L11)）求"第 0 个 / 最后一个元素"的 shader 值，是整个 crate 里"在 GPU 上取边界值"的惯用手段，段前缀和的块尾抽取与流压缩的结果长度都靠它。

## 工作组内扫描：Kogge-Stone

### 算法

[prefix_scan.rs:64](../../shader/parallel-compute/src/prefix_scan.rs#L64) 的 `build_shader`：`workgroup_size` 个线程配一块同尺寸的共享内存（[prefix_scan.rs:69](../../shader/parallel-compute/src/prefix_scan.rs#L69)），迭代 `log2(workgroup_size)` 步。核心循环（[prefix_scan.rs:84](../../shader/parallel-compute/src/prefix_scan.rs#L84)）：

```rust
iter.into_shader_iter().for_each(|i, _| {
  workgroup_barrier();

  if_by(local_id.greater_equal_than(val(1) << i), || {
    let a = value.load();
    let b = shared.index(local_id - (val(1) << i)).load();
    value.store(S::combine(a, b));
  });

  workgroup_barrier();
  shared.index(local_id).store(value.load())
});
```

- 第 i 步，`local_id ≥ 2^i` 的线程把自己的累计值加上 `shared[local_id - 2^i]` 的值（[prefix_scan.rs:87](../../shader/parallel-compute/src/prefix_scan.rs#L87)）。距离按 1、2、4、… 倍增，`log2(workgroup_size)` 步后每个线程都汇集了它之前全部元素的贡献。
- 每步两个 `workgroup_barrier`：读共享内存前一个、写回共享内存前一个（[prefix_scan.rs:85](../../shader/parallel-compute/src/prefix_scan.rs#L85)），保证第 i+1 步读到的共享值全部来自第 i 步的写回。
- 结果是 **inclusive** 前缀和：位置 i 的值包含位置 i 自身。

关键细节：无效元素先填 identity（`valid.select(input, S::identity())`，[prefix_scan.rs:78](../../shader/parallel-compute/src/prefix_scan.rs#L78)），保证整个工作组宽度上扫描是稠密的，`valid` 标志透传给下游。

### 与"单步扫描"的取舍

朴素做法是每个线程从开头一路累加，串行 O(n) 步；Kogge-Stone 用共享内存把步数压到 O(log n)，代价是工作量 O(n log n)。对工作组尺寸（几十到几百）这是共享内存带宽下最合适的选择之一。它天然是一步到位（inclusive），需要 exclusive 时用 `make_global_scan_exclusive` 整体右移。

### 局限：工作组分块

`requested_workgroup_size` 是 `workgroup_size`，而 dispatch 的组数是 `ceil(work_size / workgroup_size)`——**每个工作组独立扫描自己那一块**，块与块之间没有关联。这就是"workgroup scope"的含义，也是段前缀和存在的理由。

## 两阶段全局扫描：段前缀和

`use_segmented_prefix_scan_kogge_stone<S>(first_stage, second_stage, cx)`（[lib.rs:398](../../shader/parallel-compute/src/lib.rs#L398)）把工作组分块扫描推广成全局扫描，共 3 次 dispatch：

- **块内扫描**：按 `first_stage_workgroup_size` 做工作组内 inclusive 扫描并物化（`per_workgroup_scanned`，块内局部前缀和）。
- **块级扫描**（[lib.rs:413](../../shader/parallel-compute/src/lib.rs#L413)）：
  - `offset_access(first_stage - 1, ClampBorder, 0)` 让每个位置读到"所在块的尾元素"；
  - `stride_reduce_result(first_stage)` 按步长抽取出块尾数组（每块一个元素）；
  - 块尾数组按 `second_stage_workgroup_size` 再做一次 Kogge-Stone inclusive 扫描，`make_global_scan_exclusive` 右移一位 → 每个块的排他前缀（尾元素是全局总和）；
  - `stride_expand_result(first_stage)` 把块级前缀复制回块内每个位置。
- **合并**（[lib.rs:425](../../shader/parallel-compute/src/lib.rs#L425)）：块内 inclusive 结果与块级排他前缀逐元素 `combine`，物化输出。

### 为什么需要"段"

单个 Kogge-Stone 工作组覆盖不了大数组；两阶段结构把规模上限提到 `first_stage × second_stage`（两个 stage 都用设备上限时是 256² = 65536）。"segmented" 指"把数据切成段（块）做两阶段"，**不是**经典带段标志的 segmented scan——结果是一个普通的全局 inclusive 扫描。约束"总规模 ≤ 两 stage 乘积"是因为块尾数组必须塞进一个第二 stage 工作组；对应的断言被注释掉（[lib.rs:407](../../shader/parallel-compute/src/lib.rs#L407)），超限时结果错误，使用时需自行保证。

### 走一遍例子

设 `first_stage = 4`，输入 `[1,0,1,1, 0,1,1,0, 1,1]`（10 个元素，分 3 块，末块不满）：

- 块内 inclusive 扫描：`[1,1,2,3, 0,1,2,2, 1,2]`
- 块尾抽取（每块最后一个元素）：`[3, 2, 2]`
- 块尾 inclusive 扫描：`[3, 5, 7]`，再排他化（右移一位、边界 identity、长度 +1）：`[0, 3, 5, 7]`——多出的末位 7 正是总和
- 展开回块内（块 0 填 0、块 1 填 3、块 2 填 5，总和 7 落在数据范围之外）：`[0,0,0,0, 3,3,3,3, 5,5]`
- 合并（逐元素相加）：`[1,1,2,3, 3,4,5,5, 6,7]`——正是全局 inclusive 前缀和

注意块 0 的块级排他前缀为 0，块内扫描直接就是全局结果；这是排他边界 identity 的自然结果，无需特判。

同族的"两阶段"还有 `segmented_reduction`（[lib.rs:264](../../shader/parallel-compute/src/lib.rs#L264)）与工作组归约 `workgroup_scope_reduction`（[lib.rs:247](../../shader/parallel-compute/src/lib.rs#L247)），结构完全相同，只是把"扫描"换成"归约"。

### inclusive → exclusive

`make_global_scan_exclusive`（[lib.rs:432](../../shader/parallel-compute/src/lib.rs#L432)）是 `offset_access(-1, Const(identity), 1)`：整体右移一位、边界填 identity、长度 +1——shift 后多出的尾元素恰好是总和。radix sort 用它求"每个元素之前有几个 1"。

## 流压缩：use_stream_compaction

[stream_compaction.rs:3](../../shader/parallel-compute/src/stream_compaction.rs#L3) 的完整流程：

- **mask 转 1/0**：`filter.map(|v| v.select(1, 0))`，保留为 1。
- **inclusive 段前缀和**：`use_segmented_prefix_scan_kogge_stone::<AdditionMonoid<u32>>`，两 stage 都是设备上限（[stream_compaction.rs:19](../../shader/parallel-compute/src/stream_compaction.rs#L19)）。结果 `p_i` = 位置 i 的"全局存活序号"。
- **结果长度**：`PrefixSumTailAsSize`（[stream_compaction.rs:48](../../shader/parallel-compute/src/stream_compaction.rs#L48)）把前缀和的尾元素当作有效长度——size pass 单独跑一次（[stream_compaction.rs:21](../../shader/parallel-compute/src/stream_compaction.rs#L21)），得到"总保留数"。
- **推导排他位置与保留标志**（[stream_compaction.rs:30](../../shader/parallel-compute/src/stream_compaction.rs#L30)）：`p_prev` = 前缀和右移一位（边界 0），`keep ⇔ p_i ≠ p_prev`，`exclusive_pos = p_prev`。从 inclusive 结果反推，避免把 filter shader 再算一遍。
- **scatter**：`shuffle_move`（[lib.rs:224](../../shader/parallel-compute/src/lib.rs#L224)）让每个线程把 `source[i]` 写到 `output[p_prev]`，只有保留者写。排他位置互不重复，无原子操作、无冲突。
- **返回**：全容量 buffer + size（= 总保留数）；下游组件通过 size 界定有效长度。

## Radix sort：与流压缩的关系

[radix_sort.rs:29](../../shader/parallel-compute/src/radix_sort.rs#L29) 的 `use_device_radix_sort_naive` 按位 LSD 稳定划分，每位一趟（`S::MAX_BITS` 趟，u32 即 32 趟）：

- `is_one` 位测试 → 1/0 → 段前缀和 → `make_global_scan_exclusive` → `ones_before`（每个元素之前 1 的个数）。
- 目标位置（[radix_sort.rs:109](../../shader/parallel-compute/src/radix_sort.rs#L109)）：位为 1 → `input_size - 1 - ones_total + ones_before`（从尾部往前填）；位为 0 → `id - ones_before`（从头部往后填）。
- `shuffle_move` 落地，进入下一趟。

它和流压缩共用同一套骨架（段前缀和 + scatter），差异在目标位置公式与"全部写"（流压缩是"按保留标志写"）。当前实现是朴素的（每趟全量扫描，无内存合并优化，见 [radix_sort.rs:28](../../shader/parallel-compute/src/radix_sort.rs#L28) 的 todo）。

## 物化、缓存与动态派发

### 物化

`ComputeComponentIO::use_materialize_storage_buffer`（[abstract_component.rs:149](../../shader/parallel-compute/src/abstract_component.rs#L149)）分配一块 RW storage buffer，用 `use_and_do_write_into_storage_buffer`（[io.rs:281](../../shader/parallel-compute/src/io.rs#L281)）把组件结果写入，返回只读 view。已物化的组件（如 `DeviceMaterializeResult`）可以覆盖该方法直接暴露内部 buffer，避免一次冗余拷贝（[io.rs:132](../../shader/parallel-compute/src/io.rs#L132)）。

### 缓存

buffer 经 `cx.use_plain_state`（FunctionMemory）按帧缓存；容量不足或超过需求 2 倍时重建（[ctx.rs:84](../../shader/parallel-compute/src/ctx.rs#L84)）。这就是为什么帧内多次调用同一算法不会反复分配 GPU 内存。

### 动态派发

`work_size()` 返回 `None`（宿主侧不知道规模，例如剔除后的存活数）时，`use_dispatch_compute`（[abstract_component.rs:22](../../shader/parallel-compute/src/abstract_component.rs#L22)）走间接派发：先由 `compute_work_size`（[abstract_component.rs:63](../../shader/parallel-compute/src/abstract_component.rs#L63)）跑一个 size pass，用 `invocation_size()` 算出工作规模写入 `DispatchIndirectArgsStorage`，主 pass 再 `dispatch_workgroups_indirect`。`DeviceMaterializeResult.size`（`Vec4<u32>`，x 为有效长度）就是这条链路的产物，作为下游组件的 `invocation_size` 来源。

## 下游消费

### draw-list 的 GPU 剔除流压缩

`DeviceDrawList::use_culled_list_and_do_culling`（[shader/draw-list/src/stream_compact/mod.rs:11](../../shader/draw-list/src/stream_compact/mod.rs#L11)）：predicate 生成 1/0 mask → `use_segmented_prefix_scan_kogge_stone`（两 stage 均为 `max_compute_invocations_per_workgroup`，[mod.rs:40](../../shader/draw-list/src/stream_compact/mod.rs#L40)）→ `SegmentedListScatter` 分段 scatter 并回写每段新 count 与前缀和。总容量受 256² 约束。完整讲解见 [draw-list-guide.md](draw-list-guide.md) 的「stream_compact：GPU 流压缩」一节。

### midc_downgrade 降级管线

不支持 `MULTI_DRAW_INDIRECT_COUNT` 的平台（[platform/graphics/webgpu-midc-downgrade/src/lib.rs:20](../../platform/graphics/webgpu-midc-downgrade/src/lib.rs#L20)）把整段多绘制降级为单段间接绘制：对所有子列表的顶点数做一次段前缀和（[lib.rs:112](../../platform/graphics/webgpu-midc-downgrade/src/lib.rs#L112)），再单 pass 写每个子列表的段内排他前缀与 `DrawIndirect` 参数（[lib.rs:171](../../platform/graphics/webgpu-midc-downgrade/src/lib.rs#L171)），前缀和 buffer 按 offset 对齐切 view 供顶点阶段使用。接线在 [scene/rendering/gpu-base/src/mid/mod.rs:84](../../scene/rendering/gpu-base/src/mid/mod.rs#L84) 的 `use_and_create_default_indirect_draw_provider`（`enable_midc_downgrade` 为真时包 `MIDCDowngradeBatch`，见 [midc_downgrade.rs:26](../../scene/rendering/gpu-base/src/mid/midc_downgrade.rs#L26)）。机制详见 [indirect-draw-command-guide.md](indirect-draw-command-guide.md) 的「MIDC 降级机制」。

### occlusion-culling 的批次拆分

[scene/rendering/occlusion-culling/src/lib.rs:76](../../scene/rendering/occlusion-culling/src/lib.rs#L76) 对同一个 batch 连做两次 `use_culled_list_and_do_culling`（`filter_last_frame_visible_object(...)` 与其 `not()`），把"上一帧可见/不可见"拆成两半分别绘制与测试，见 [occlusion-culling-guide.md](occlusion-culling-guide.md)。

帧内统一入口是 `FrameCtx::access_parallel_compute`（[ctx.rs:230](../../shader/parallel-compute/src/ctx.rs#L230)），它把 `FrameCtx` 的 encoder 与 FunctionMemory 包成 `DeviceParallelComputeCtx` 交给算法代码。

## GPU 单元测试模式

每个模块内都带 `#[pollster::test]` 测试，模式统一：

- 环境：`gpu_cx!(cx)` 宏（[ctx.rs:245](../../shader/parallel-compute/src/ctx.rs#L245)）构造真实 GPU、encoder 与 `DeviceParallelComputeCtx`。
- 上载：`slice_into_compute(&input, cx)`（[io.rs:44](../../shader/parallel-compute/src/io.rs#L44)）把 host 切片变成物化组件。
- 断言：`run_test` / `run_test_with_size_test`（[lib.rs:142](../../shader/parallel-compute/src/lib.rs#L142)）读回 buffer 逐元素比较，并可断言有效长度。它把同一计算跑两遍——直接派发与 `force_indirect_dispatch = true`（[lib.rs:173](../../shader/parallel-compute/src/lib.rs#L173)）——保证两条派发路径行为一致。

可参考的实例：[test_stream_compaction](../../shader/parallel-compute/src/stream_compaction.rs#L101)（输入 `[1,0,1,0,1,1,0]` 期望 `[1,1,1,1,0,0,0]`，有效长度 4）、[test_prefix_sum_kogge_stone](../../shader/parallel-compute/src/prefix_scan.rs#L153)（70 个 1 的全局 inclusive 扫描）、radix sort 测试（[radix_sort.rs:144](../../shader/parallel-compute/src/radix_sort.rs#L144)）。draw-list 的剔除测试（[shader/draw-list/src/stream_compact/tests.rs](../../shader/draw-list/src/stream_compact/tests.rs)）是下游组合测试的范本。

## 使用模板

### 模板一：直接使用流压缩

```rust
// input 是 ComputeComponentIO<T>（host 上载或来自其他 GPU 组件）
let filter = input.clone().map(|v| v.equals(1)); // ComputeComponentIO<bool>
let result = input.use_stream_compaction(filter, cx); // DeviceMaterializeResult<T>
// result.buffer 是压缩后的全容量 buffer，result.size 的 x 分量是有效长度
```

[stream_compaction.rs:101](../../shader/parallel-compute/src/stream_compaction.rs#L101) 的测试即此模板。

### 模板二：手动组合"扫描 + 位置 + scatter"

```rust
// 段前缀和（inclusive），两 stage 都要 ≤ 设备上限、乘积 ≥ 总规模
let inclusive = data
  .use_segmented_prefix_scan_kogge_stone::<AdditionMonoid<u32>>(g1, g2, cx);
// 排他化
let exclusive = inclusive.clone().make_global_scan_exclusive::<AdditionMonoid<u32>>();
// 用排他前缀做位置（如双调重排、radix sort 的目标位置公式）
// 物化 + shuffle_move 落地
let moved = exclusive.use_materialize_storage_buffer(cx).shuffle_move(shuffle_idx, cx);
```

radix sort 的每趟划分（[radix_sort.rs:46](../../shader/parallel-compute/src/radix_sort.rs#L46)）是完整范例。

### 模板三：自定义 monoid

实现 `DeviceMonoidLogic`（[prefix_scan.rs:10](../../shader/parallel-compute/src/prefix_scan.rs#L10)）即可复用全部扫描/归约/压缩原语，例如最大值扫描或自定义结构体的结合运算。

### 模板四：接入帧管线

```rust
frame_ctx.access_parallel_compute(|cx| {
  let batch = batch.use_culled_list_and_do_culling(cx, Box::new(culler));
  // …
});
```

见 [application/viewer-content/src/rendering/culling.rs:123](../../application/viewer-content/src/rendering/culling.rs#L123) 的完整剔除链路。

### 约束与注意点

- 段前缀和的总规模不得超过 `first_stage × second_stage`；断言未启用，超限不会报错。
- `shuffle_move` 需要可随机读写的物化输出（[shuffle_move.rs:6](../../shader/parallel-compute/src/shuffle_move.rs#L6)）。
- `use_stream_compaction` 的输出是"全容量 buffer + size"，下游读取必须按 size 截断，不要按 `item_count` 遍历。
- EDSL 依赖线程局部状态，组件构建与组合不能跨线程（见 shader-edsl-core 的注意项）。
- 自定义组件的 `ShaderHashProvider` 必须覆盖所有影响 shader 的输入（闭包捕获值、标志位），否则管线缓存键缺项。

## 延伸阅读

- 组件与派发机制：[shader/parallel-compute/src/abstract_component.rs:3](../../shader/parallel-compute/src/abstract_component.rs#L3)、[abstract_invocation.rs:5](../../shader/parallel-compute/src/abstract_invocation.rs#L5)
- 位置变换组合子：[shader/parallel-compute/src/access_behavior.rs:59](../../shader/parallel-compute/src/access_behavior.rs#L59)、[stride_read.rs:3](../../shader/parallel-compute/src/stride_read.rs#L3)
- 归约与直方图：[shader/parallel-compute/src/reduction.rs](../../shader/parallel-compute/src/reduction.rs)、[histogram.rs](../../shader/parallel-compute/src/histogram.rs)
- 剔除流压缩的完整消费链：[draw-list-guide.md](draw-list-guide.md)（stream_compact 一节）、[indirect-draw-command-guide.md](indirect-draw-command-guide.md)（MIDC 降级机制）
- 遮挡剔除对压缩原语的组合使用：[occlusion-culling-guide.md](occlusion-culling-guide.md)
