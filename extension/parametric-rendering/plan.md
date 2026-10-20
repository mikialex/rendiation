# 参数曲面 GPU 自适应细分渲染计划

背景调研见 [research.md](https://github.com/mikialex/notebook-public/blob/master/project/3d-parametric-rendering/research.md)。
本计划走 A-A-B 方向（GPU 上递归拆分 patch，自适应三角化），参考 Eisenacher 2009
（Real-Time View-Dependent Rendering of Parametric Surfaces）和 Unterguggenberger 2024
（Fast Rendering of Parametric Objects on Modern GPUs），不采用 ETER 的整 patch 细分等级估计。

## 目标与范围

目标：给定 `ParametricNormalizedModel`（有理 Bézier patch + instance）和相机，每帧在 GPU 上生成视角相关的
三角化并绘制，轮廓误差不超过给定的屏幕空间阈值（默认 0.5 px），同一个 face 内的 patch 之间没有缝。

本阶段不做：

- trim（裁剪环不上传、不使用；但输出接口为按片元裁剪预留 `(patch_id, u, v)`）
- 跨 face 的 watertight（依赖 trim 边的处理，见"后续阶段"）
- 遮挡剔除（HiZ）
- 帧间增量更新（每帧重建）

## 输入数据现状

数据来自 `extension/parametric-data-normalization`（`parametric-normalized-model` crate，`.pnm` 文件）。
steam-controller 的统计：

| 项 | 数据 |
| --- | --- |
| patch 数 | 17387，其中带裁剪的 9969（57%），裁剪曲线 41466 条 |
| 阶数 (u,v) | (5,5) 9324，(3,3) 6225，(3,5)/(5,3) 774，其余零散，最高 5 |
| 有理 patch | 2355（13.5%），单 patch 内权重最大比值 1.97 |
| patch 包围盒对角线 | p10 0.75，p50 2.0，p90 6.1，最大 89（模型单位 mm） |
| 控制网相对角点双线性插值的偏差 / 对角线 | p50 1.5%，p90 24%，max 48% |
| 同 face 相邻 cell 的共享边界控制点 | 26377 对同阶相邻 cell，全部逐位相同 |

结论：

- 大部分 patch 很小，常规视距下第 0 层就是叶子，开销主要在每个 patch 输出多少三角形，而不在递归深度。
- 不做 trim 时 steam 有一半以上的 patch 会画出整个 cell，画面是错的。本阶段的可视化验证应使用
  未裁剪的 patch（`TrimmedBezierPatch::is_untrimmed`）或合成场景。
- 同 face 内共享边逐位相同，是下文边吸附 watertight 方案成立的前提。周期方向的 seam（最后一个
  cell 与第 0 个 cell）和跨 face 的情况尚未验证。

现有 GPU 代码（`src/device`）：f64 模型类型到 GPU 结构的转换（`bezier_surface_to_gpu` /
`bezier_curve_to_gpu`），Bernstein 求值（阶数上限 `MAX_GPU_DEGREE` = 14）和 degree ≤ 3 的
de Casteljau 快速路径，以及与 CPU f64 求值对比的 GPU 测试。这些在新管线里复用。

## 总体架构

```text
CPU 上传: patch 控制点(相对 face 原点, f32) + patch 元数据(阶数, 法线锥) + instance 变换
  -> [每帧] 控制点变换到相机空间; 每个 (instance, patch) 产生一个根工作项
  -> [每层几次 dispatch, 最多 MAX_LEVEL 层]
       对每个工作项: 剔除 -> 计算误差 -> 拆成 4 个子工作项 或 输出叶子(带 k)
  -> [顶点 pass] 叶子网格顶点写入顶点池（边界顶点定位到共享的边线性化上）
  -> 按 k 分桶的 draw_indirect: instance = 叶子, vertex shader 从顶点池读取
  -> fragment: 第一版用顶点法线, 之后用 (patch_id, u, v) 求偏导得到逐像素法线
```

## 关键设计

### 子 patch 用二进位 key 表示

工作项和叶子只存 `(instance_patch_id, level, ix, iy)`（两个 u32 即可），表示 patch 参数域上的
`[ix, ix+1] / 2^level × [iy, iy+1] / 2^level`。

- 不在队列里存子 patch 控制点：(5,5) 阶 36 个 vec4 = 576 字节，带宽不可接受。每层需要时从原始
  控制点用 de Casteljau 提取子 patch。
- 所有最终顶点都用原始控制点在全局 `(u, v)` 上求值，不累积拆分带来的舍入误差。二进位参数在 f32
  里是精确的，这是共享边逐位一致的前提。
- 以后做帧间增量（参考 Dupuy 的 concurrent binary tree：每帧每个节点最多拆或合一层）可以直接复用
  这个表示。

### 拆分判据（屏幕空间几何误差）

工作项拆分的条件：内部误差超阈值，或者四条边中任一条的边误差超阈值（见"边细分与 watertight"），
或者叶子网格无法在 `K_MAX` 内满足误差，或者屏幕投影尺寸超过三角形最大尺寸上限。

几何误差：

- 多项式 patch：子 patch 控制点与"角点双线性插值升阶到同阶后的控制点"逐个比较，最大距离是曲面到
  双线性插值的严格上界（Eisenacher 的判据）。
- 有理 patch：逐个比较不是严格上界，需要一个与权重比相关的修正系数。具体上界待推导（见"未决
  问题"），推导结果必须用 CPU 稠密采样验证。数据里有理 patch 占 13.5%，权重比 ≤ 2，影响可控。
- 投影到屏幕：误差 × 焦距 / 子 patch 包围盒的最近深度（保守）。递归后包围盒变紧，近处不会过度保守。
- 包围盒跨近平面：强制拆分，直到达到 `MAX_LEVEL`（此时按最近深度为近平面处理）或被剔除。

着色精度不由细分保证（见"渲染"），判据只管几何误差，另加一个三角形最大屏幕尺寸上限
（初值 32 px，可配置），保证透视插值 uv 的质量。

### 叶子输出 k×k 网格

叶子不只输出一个 quad，而是 k×k 的规则网格，k ∈ {1, 2, 4, 8, 16}（`K_MAX` = 16，可调）。

- 队列流量降到 1/k²，递归层数减少 log2 k。
- k 的选择用二阶导数上界：多项式 Bézier 的二阶偏导上界是 `n(n-1)·max|Δ²b|`（按方向分别计算，
  混合项用 `n·m·max|Δ_uΔ_v b|`），网格插值误差约为
  `1/8·(h_u²·M_uu + 2·h_u·h_v·M_uv + h_v²·M_vv)`（Filip, Magedson, Markot 1986，常数需按实际的
  三角形划分核对）。取满足阈值的最小二的幂 k；若 k > `K_MAX` 则继续拆分。
- 这是对递归后叶子的上界，不是 ETER 那种对整个 patch 的估计，局部自适应性由递归保证。
- u、v 两个方向的 k 可以不同（各向异性 patch 很多，如细长的圆角带），第一版先用 `k_u = k_v`，
  数据验证后再决定是否分开。

### 边细分与 watertight

相邻叶子拆分深度不同会产生 T 型接缝。方案：边的细分只由边自己决定，叶子的边界顶点吸附到边的
"最粗平坦段"上，不需要邻接信息。（下面是最初的单弦版本，阶段 0 已改为最多 `K_MAX` 段的折线、
iso 线根段限定到所在层级的 cell、顶点位置递归定位，见"阶段 0 结果"。）

- 边：原始 patch 的一条 iso 线，包括四条边界和内部 `u = 常数` / `v = 常数` 的二进位 iso 线。
  边的根段是这条 iso 线在原始 patch 内的完整参数范围。
- 边测试：只用边曲线本身（1D 有理 Bézier，从原始控制点提取）和相机，判断子段相对其弦的屏幕误差。
- 最粗平坦段 F：对叶子的一条边 S，从边的根段向 S 二分下行，第一个通过边测试的段就是 F。
  因为拆分判据要求叶子的边通过边测试，所以 F ⊇ S；落在 F 内的所有叶子（两侧）都会走过相同的
  路径，得到同一个 F。
- 吸附：叶子边界上的顶点放在 `lerp(E(F.a), E(F.b), t_local)` 上，F 的两个端点用边曲线精确求值；
  叶子内部顶点在曲面上精确求值。两侧所有边顶点落在同一条线段上，几何上无缝。
- 边方向规范化：同一条边在两侧必须用相同方向和相同参数计算（比如按两端点控制点的字典序决定
  方向），否则 `lerp(A, B, t)` 与 `lerp(B, A, 1 - t)` 舍入不同。同一个 face 的相邻 cell 共享边在当前
  数据里方向一致且逐位相同；周期 seam 需要额外验证。
- 每个叶子每条边做一次 F 查找（O(level) 次边测试），边上所有顶点共用。

已知风险：T 型连接处顶点虽然在线段上，但 lerp 的舍入加上光栅化规则仍可能产生零星的漏像素。
如果实测可见，改为"边顶点集合只由 F 决定"（F 按自己的边误差均匀二分成 m_F 段，叶子的边只取落在
自己范围内的 F 分段点），代价是叶子边界的顶点数要与网格 k 解耦。

这个设计把"边细分"作为独立概念。后续 trim 边（每条裁剪曲线带 `EdgeId`，两侧共享同一条 3D 边）
可以走同一套逻辑，实现跨 face 的 watertight（WATER 的方向）。

### 渲染

阶段 1 的实现以"GPU 调度"一节为准，这里只保留方向：

- 叶子按 k 分桶，每桶一次 `draw_indirect`：instance 数 = 该桶叶子数，顶点数 = `k·k·6`，不生成索引。
  顶点位置和法线在 compute pass 里预先算好写进顶点池（边界顶点的定位需要多次边测试和嵌套插值，
  不适合放进 vertex shader；同一组 compute pipeline 算出的位置才保证共享顶点逐位相等），vertex
  shader 由 `(instance_index, vertex_index)` 找到叶子和格点，从顶点池读取。
- 第一版用顶点法线。后续把 `(patch_id, u, v)` 作为 varying 传到 fragment，用曲面偏导逐像素计算法线，
  解决"轮廓精确但着色不精确"的问题，代价是每个片元一次曲面求导，需要实测。
- 后续 trim 直接在 fragment 用 `(patch_id, u, v)` 判断裁剪并 discard（research 中 Direct trimming 的
  方向，或者把裁剪曲线统一近似到二次再用 slug 类算法），细分部分不用改。

### 剔除

- 视锥：子 patch 的包围盒（控制点凸包的包围盒，有理情况下对正权重同样成立）。
- 背面：预计算每个原始 patch 的法线锥（The Cone of Normals），子 patch 第一版直接继承父 patch 的
  法线锥（保守）。
- 遮挡剔除（两阶段 HiZ）后续再做。CAD 模型内部结构多，预期收益大。

### 精度

GPU 上是 f32。控制点存成相对 face 原点（face 控制点包围盒中心）的 f32 坐标，CPU 每帧用 f64 合成
`view · instance.transform · translate(face_origin)` 再转 f32，变换 pass 用它把控制点变到相机空间。
同一个 face 内共享边的 f64 控制点逐位相等，减去同一个原点、乘同一个矩阵后在 f32 里仍然逐位相等。
（不能用 patch 中心：各 patch 中心不同会破坏共享边的逐位相等。）

## GPU 调度

原则：多用 dispatch、用一些带宽换简单的 kernel，中间结果走 buffer；但 dispatch 总数要有界（每帧约
80 次以内）。所有尺寸都由 indirect 参数驱动，每帧没有 CPU 读回（统计和溢出标志异步读回，下一帧
生效）。WebGPU 没有前向进度保证，不使用 persistent thread。

```text
[静态] CP 池(f32, face 原点相对, 紧密排列) + patch 表(阶数, CP 偏移, 是否裁剪)
       + item 表(instance, patch, 相机空间 CP 偏移)
[每帧 CPU] instance 矩阵(f64 合成 → f32) + uniform(焦距, 视口, near, 阈值, K_MAX, MAX_LEVEL)
T    变换：每个 CP 一个线程，写相机空间 CP 池
I    初始化：每个 item 一个根 key 写入队列 A，写第 0 层的 indirect 参数
每层 L = 0..=MAX_LEVEL（ping-pong 队列，4 次 dispatch + 1 次参数 pass）：
  L.a 提取   每个 (工作项, 控制点) 一个线程，子 patch 控制点写入子 patch buffer
  L.b 内部   每个工作项一个线程：包围盒、剔除、k_grid 或拆分原因
  L.c 边     每个 (工作项, 边) 一个线程：flat_ancestor，得到这条边要求的 k 或失败
  L.d 决策   每个工作项一个线程：追加 4 个子 key 或输出叶子
               叶子：追加叶子记录，写入 k 桶，在顶点池 bump 分配 (k+1)² 个顶点
  L.p 参数   单线程：写下一层的 indirect 参数，交换计数
V    顶点（正向 R 轮 + 反向 R 轮 + 内部点 1 次）：
  第 0 轮的请求 = 所有叶子边界顶点，请求下标 = 顶点池下标
  正向第 r 轮：每个请求一个线程，在两条 iso 线上做 locate
                 不在段内部 → 精确求值，写结果；在段内部 → 记录 {t, child_a, child_b}，
                 a、b 追加为第 r+1 轮请求
  反向第 r = R-1..0 轮：value = lerp(value[child_a], value[child_b], t)
  内部点：每个叶子内部顶点一个线程，曲面求值；法线同时由偏导算出
  R = MAX_LEVEL + log2 K_MAX，空轮次 dispatch 0 个 workgroup
D    收尾：写 5 个桶的 draw_indirect 参数 {6k², 桶内叶子数} 和统计
绘制 每个 k 桶一次 draw_indirect
```

- 嵌套插值是一条链：阶段 0 测得每个顶点展开后的精确求值项数 = 嵌套深度 + 1（steam 最坏 7），
  所以按轮展开的冗余很小，不需要去重。
- `locate` / `flat_ancestor` 内部保留不超过 `MAX_LEVEL` 次的有界循环，不再拆 dispatch。
- 容量：所有队列、子 patch buffer、叶子、桶、顶点池、请求池都预分配固定容量，原子计数超出时丢弃
  并置溢出标志，不允许越界写。读回后扩容，下一帧生效。
- 达到 `MAX_LEVEL` 仍不满足判据的工作项按 `K_MAX` 输出叶子并计数。
- 子 patch 走 buffer 后 kernel 不受 private 数组大小限制，最高阶数由子 patch buffer 的单项容量决定，
  第一版按 `MAX_GPU_DEGREE` 分配；超过的 patch 在上传时拒绝并计数。

## 实施阶段

### 阶段 0：CPU 参考实现

位置：新 crate `extension/parametric-data-normalization/parametric-tessellation`（只依赖
`parametric-normalized-model`），f64 实现与 GPU 完全相同的算法（相同的 key、判据、k 选择、F 查找、
吸附）。验证：

- 误差上界：对随机和真实 patch，叶子网格与曲面稠密采样的最大偏差（投影到屏幕）不超过阈值。
  这里同时验证有理修正系数。
- watertight：所有相邻叶子共享边上的顶点逐位一致（同 face 内），并统计 T 型连接数量。
- 统计：叶子数、三角形数、层数分布、k 分布，与均匀细分（Unterguggenberger 风格）对比三角形数。
- CLI：`parametric-normalization-cli` 增加 `--camera` 选项，输出视角相关的 glb 供肉眼检查（第一版
  只输出未裁剪 patch，或者全部输出但给裁剪 patch 标颜色）。

#### 阶段 0 结果

实现在 `parametric-tessellation`，CLI：`convert x.pnm -o view.glb --camera fit[:ZOOM] --verify`。
相对上文设计的修正（细节见 crate 文档）：

- 边线性化改为最多 `K_MAX` 段的均匀折线：F 是第一个能用 m_F ≤ `K_MAX` 段满足边误差的段，叶子把
  k 提高到包含自己边内的所有折点。单弦方案下有弯曲边的叶子必须拆到边变直，k > 1 没有意义。
- iso 线 `u = c` 的根段限定为 c 所在层级的 cell，而不是整条 patch 边。这样一个点最多落在它两条
  iso 线之一的某段内部，顶点位置成为点的纯函数：在那条线的段端点（递归定位）之间插值，否则精确
  求值。段端点直接精确求值会在"T 型点恰好是垂直线段端点"处开裂。
- 有理 patch 不需要修正系数：`(H - B·w)` 是 (n+1, m+1) 阶多项式，系数最大值 / `w_min` 就是严格
  上界；二阶导上界用商法则推导。两者都有随机 patch 的稠密采样测试。
- `edge_error_share` 取 0.5 时边从不需要抬高 k，三角形更少；0.25 时约 1/3 的叶子被边抬高，
  三角形多 1.6～2 倍。严格上界是 内部 + 嵌套插值深度 × 边，但实测误差始终低于阈值。
- 背面剔除（法线锥）尚未实现。

steam-controller，1920×1080，fov 45°，0.5 px，`K_MAX` 16：

| 视角 | 叶子 | 最大层 | 三角形 | 均匀网格对比 | 实测最大误差 | 顶点插值嵌套深度分布 |
| --- | --- | --- | --- | --- | --- | --- |
| fit | 17471 | 1 | 716k | 245k（0.34×） | 0.31 px | [254k, 154k, 240] |
| fit:4 | 10875 | 4 | 1.50M | 766k（0.51×） | 0.31 px | 最深 4 |
| fit:20 | 3635 | 5 | 435k | 15.0M（34×） | 0.35 px | 最深 5 |
| fit:80 | 3045 | 6 | 453k | 4.4M（9.8×） | 0.31 px | 最深 6 |

所有视角下共享边顶点 0 个不一致，T 型点到对侧边的距离 ≤ 3.5e-13 px。远景下自适应比均匀网格
三角形多，原因是 k 只取 2 的幂（约 1.45×），以及误差预算要分给边（内部 0.25 px 对均匀网格的
0.5 px，约 2×）；近景下均匀网格对跨近平面附近的 patch 失效。

对阶段 1 的影响：

- 嵌套插值深度可达 6，每层要做 O(level) 次边测试，不适合放在 vertex shader 里递归。考虑在
  compute pass 里给每个叶子边预先算好折点位置，或者限制深度。
- GPU 上控制点相对 patch 中心存储会破坏跨 patch 共享边的逐位相等，需要改成按 instance（或 face）
  的公共原点。

### 阶段 1：GPU 移植

位置：`rendiation-parametric-rendering` 的 `tessellation` 模块，shader EDSL 实现上节各 pass。

- M0 CPU 参考补齐：每个判定记录裕量 `|bound·s − tol| / tol`；`position` 改成与 GPU 相同的正向展开
  / 反向回代结构，并统计每轮请求数；`check_watertight` / `measure_error` 接受外部给的顶点位置；
  face 原点处理。
- M1 数据上传：打包的 CP 池、patch 表、item 表、每帧矩阵与 uniform（现有 `GpuBezierControlPoints`
  每个 patch 固定 225 个 vec4，不能用）。测试：读回后与 CPU 数据逐项比对。
- M2 shader 函数库：dyadic 参数运算（精确相等）、子 patch / 子曲线提取、`surface_bounds` /
  `curve_bounds`、iso 线提取（边界线方向规范化）、`linearization` / `flat_ancestor` / `locate`。
  每个都和 CPU f64 版本做随机对比测试。
- M3 细分 pass（T、I、L.a–L.p、D）：读回叶子集合与 CPU 参考做集合比对，不一致的必须全部落在
  小裕量的判定上；比对每层工作项数、k 分布；测试小容量下的溢出路径。
- M4 顶点 pass（V）：读回顶点池跑 watertight 检查，共享顶点不一致数必须严格为 0（GPU 内的确定性），
  T 型连接偏离在 f32 舍入量级；误差检查对 CPU f64 曲面不超过阈值加 f32 噪声。
- M5 绘制组件：按桶 draw_indirect，从顶点池读取，顶点法线 + 头灯着色，可切换按 k 着色，带裁剪的
  patch 调淡。
- M6 统计读回与溢出扩容。

GPU 计算与绘制之间的接口是每帧的输出 buffer（`TessellationFrameOutput`：顶点池、叶子记录、
各桶叶子下标、draw 参数）。绘制组件先用 CPU 参考的结果上传填充这些 buffer 来开发和验证，GPU
计算完成后替换生产者。

风险：FMA 合并等编译差异可能让细分 pass 和顶点 pass 对同一条边的判断不同（影响 k 是否包含所有
折点），M4 的 watertight 测试专门检查这一项；dispatch 数量和每轮 buffer 带宽在 M3 / M4 实测。

#### 阶段 1 进展

M0–M4、M6 完成（M5 绘制由另一条线做）。实现在 `src/tessellation`：`data`（静态上传、每帧矩阵与
uniform）、`shader_lib`（与 CPU 逐操作对应的 shader 函数）、`passes`（每帧的全部 compute pass）、
`tessellator`（`GpuTessellator`、异步统计与扩容、测试用读回）。CPU 参考的补充见
`parametric-tessellation` 的 README。

```rust
GpuTessellator::new(&gpu, &model, config);
t.set_config(config);
t.set_untrimmed_only(true);
let output = t.encode_frame(&gpu, &mut encoder, &camera); // 提交后可绘制
t.last_stats(); // 更早一帧的统计，溢出后下一帧自动扩容
```

相对上文设计的改动：

- 没有单独的 L.p：L.d 追加子 key 时用 atomic max 直接写下一层的 indirect 参数（每层一个参数 buffer，
  两组参数：L.a 用 `(x, 子 patch 跨度, 1)`，L.b–L.d 用 `(x, 1, 1)`，L.c 的 workgroup 是 64×4）。
  顶点轮次同样由上一轮写下一轮的参数。默认配置（`MAX_LEVEL` 10，`K_MAX` 16）每帧 77 次 dispatch。
- 第 0 轮和之后的轮次用同一条 pipeline（按 uniform 分支取请求），精确求值和 lerp 各只在一条
  pipeline 的一处代码里，保证共享点逐位一致。Metal 默认开 fast math，不同 pipeline 之间不能假设
  逐位一致。请求池是一个共享池，每轮的起点由前面各轮的计数求和得到。
- 子 patch buffer 的单项容量取上传的 patch 里最大的控制点数（steam 为 36），不是 `MAX_GPU_DEGREE`。
- 法线用 face 原点相对的控制点求偏导再乘实例矩阵的线性部分（相机空间 f32 下远处的小 patch 偏导
  误差可达 5%）；极点处（偏导数值为零）改在叶子内部 1/64 处取法线。CPU 参考在极点处法线是噪声。
- CPU 参考也改为 face 原点相对的控制点；watertight 检查只在同一个 instance（face 的一次出现）内
  识别共享边：不同 face 的边界控制点在 f64 里可能恰好逐位相同，但各自减去不同原点后在 f32 里不同，
  跨 face 本阶段不处理。
- 每个 shader stage 最多用 10 个 storage buffer，超过 WebGPU 默认的 8（native 适配器没问题）。

steam-controller，1920×1080，fov 45°，0.5 px，`K_MAX` 16，Apple M5 Pro（叶子不一致数是两侧之和）：

| 视角 | 叶子 CPU / GPU | 不一致叶子（最大裕量） | 共享顶点 / 不一致 | T 型点最大偏离 | 最大误差 | 每帧 |
| --- | --- | --- | --- | --- | --- | --- |
| fit | 17471 / 17471 | 14（2.9e-3） | 86933 / 0 | 1.2e-4 px | 0.29 px | 5.3–6.7 ms |
| fit:4 | 10875 / 10875 | 4（1.6e-3） | 101519 / 0 | 1.3e-4 px | 0.31 px | 6.5–6.7 ms |
| fit:4 仅未裁剪 | 4052 / 4052 | 4（1.6e-3） | 20163 / 0 | 1.3e-4 px | 0.27 px | 5.3–6.6 ms |
| fit:20 | 3635 / 3635 | 2（8.6e-4） | 31742 / 0 | 1.6e-4 px | 0.36 px | 5.3–6.6 ms |
| fit:80 | 3045 / 3045 | 0 | 30270 / 0 | 1.6e-4 px | 0.31 px | 5.3 ms |

每帧时间是 encode + submit + 等待完成的墙钟时间（CPU encode 约 0.1 ms），相机背对模型（第 0 层
全部剔除）时约 3.3 ms，即 77 次 dispatch 加变换、第 0 层提取和包围盒的固定开销。CPU 参考单线程
细分约 27 ms（fit:4，不含顶点）。每层工作项数与 CPU 完全一致，k 分布只差不一致的叶子。合成场景（含跨近
平面、`MAX_LEVEL` 5 的强制叶子、0.2 px / 边份额 0.25 / `K_MAX` 8）叶子集合与 CPU 完全一致。

- 不一致的叶子全部落在相对裕量 < 3e-3 的判定上。f32 的误差主要来自相机空间坐标的抵消（距离 250
  的 patch 的二阶差分），不是 f32 精度本身。
- 同一个叶子顶点与 CPU 位置的偏差一般 < 6e-4 px；个别顶点到 0.24 px，是边线性化判定在阈值附近
  翻转（把 CPU 的阈值改变 1e-3 即得到 GPU 的位置，偏差降到 1e-5 px），在误差预算之内。
- 细分 pass（L.c）与顶点 pass 对同一条边的判断一致：如果不一致，T 型点会偏离一个边容差量级
  （约 0.1 px），实测最大 1.6e-4 px，属于 f32 舍入。
- 溢出：小容量下丢弃写入并置标志，异步读回后扩容（每次至少翻倍），十几帧收敛到与不受限时相同的
  叶子集合（测试 `overflow_grows_buffers`）。

未解决：

- bound 在相机空间计算，远处 patch 有抵消误差（判定裕量到 3e-3）。可以把差分改在 face 原点相对
  坐标里算再旋转。
- 统计读回依赖 `GPU::submit_encoder` 触发；同一次提交前多次 `encode_frame` 会互相覆盖 uniform 与
  矩阵（queue 写入）。
- `MAX_LEVEL` 或 `K_MAX` 增大时 dispatch 数增加（每层 4 次，每轮 2 次），20 层时约 120 次。
- 背面剔除（法线锥）、跨 face watertight、HiZ 仍未做。

### 阶段 2：接入 viewer

- 像 `gltf_io` 一样，通过拖放或文件对话框加载 `.pnm`。
- 独立的 renderer 画在主场景 pass 里，参与深度测试，顶点法线 + 头灯简单着色，不接入材质、光照、
  拾取。用 viewer 相机的 view（f64）和 projection。
- UI 面板：阈值、`K_MAX`、`MAX_LEVEL`、最大三角形尺寸；着色模式（法线 / 按 k / 按叶子）；
  只画未裁剪 patch 的开关；统计（叶子、三角形、每层工作项、溢出）。
- 生产者可切换：GPU 计算 / CPU 参考（CPU 参考在相机变化时重算并上传，用于对照和兜底）。

#### 阶段 2 进展

已接入 viewer，默认生产者是 GPU 细分器（`GpuTessellationProducer`），也可以切换到 CPU 参考对照：

- 加载：拖放 `.pnm`、"Parametric Model" 窗口的 load 按钮（文件对话框）、`load-pnm [path]` 命令，
  启动时自动加载用环境变量 `RENDIATION_VIEWER_PNM=/path/x.pnm`（或 app 配置 `auto_load_pnm`）。
  解析在 worker 线程，一次一个模型，加载后相机自动对准模型（按钮或 P 键重新对准）。
- 绘制组件（M5）：`src/tessellation/draw.rs`，每个 k 桶一次 `draw_indirect`，桶号编进管线（5 条管线），
  不剔除背面（三角形绕向跟随参数域，不跟随面朝向），头灯 lambert 取绝对值双面着色。着色模式：灰、
  按 k、按叶子、按 patch；带裁剪的 patch 默认调淡，可切换为正常或隐藏（隐藏在 vertex shader 里把
  顶点移出裁剪空间）。
- viewer 侧（`application/viewer-content/src/rendering/parametric.rs`）：在主场景 pass 之后、MSAA
  resolve 之前单独一个 pass，用同一组 color / depth 附件（MSAA、reverse z 与场景一致），加上 TAA
  抖动。不写 entity id 和 g-buffer 法线，所以不参与拾取、描边和 SSAO；在透明物体之后绘制。
- 输出在生产时相机的相机空间，绘制时用 `当前 view · 生产时 view⁻¹`（f64 合成）映射到当前视图，
  所以生产者落后于相机时位置仍然正确，只是细分等级不匹配。多个视口共用第一个视口的细分结果。
- 生产者抽象 `TessellationProducer`（`create_tessellation_producer` 按类型创建）。GPU 生产者在请求
  （相机、配置、过滤）变化或上次统计报告溢出时把细分编码进帧 encoder，其余帧只轮询统计、继续画上一次
  的输出。CPU 参考在请求变化且没有任务在跑时开一个线程，同时最多一个任务，期间继续画上一次的结果。
  冻结细分后可以从别的角度检查细分结果。
- 正交相机不更新细分（只画上一次的结果）。模型单位不换算，可用 model scale 缩放。

steam-controller（debug 构建，对准后 1600×1200）：17486 叶子、71.4 万三角形，细分约 0.55 s、构建
输出约 1.6～2.2 s（release 下细分约 20～30 ms）。离屏测试 `application/viewer-content/tests/parametric_offscreen.rs`
覆盖 MSAA / TAA / 隐藏裁剪 patch 三种情况（GPU 生产者），以及 CPU 参考，并检查两者的覆盖率一致。
release 下 steam 用 GPU 生产者：17387 叶子、29.7 万三角形（CPU 参考 29.7 万，差异来自阈值附近的
判定），覆盖率与 CPU 参考相同。

### 后续阶段

- trim：fragment 级裁剪。
- 跨 face watertight：trim 边按"边细分"逻辑统一处理。
- HiZ 遮挡剔除。
- 帧间增量（concurrent binary tree 式的拆 / 合）。

## 未决问题

- 有理 patch 的误差上界（控制网判据的修正系数，以及 k 选择用的二阶导数上界）。先推导，再用阶段 0
  的稠密采样验证；如果严格上界太保守，考虑用 `w_max / w_min` 修正的近似并以测试覆盖。
- 周期 seam 处共享边是否逐位相同、方向是否一致；跨 face 的边不在本阶段范围内。
- k 是否需要 u、v 分开；`K_MAX`、`MAX_LEVEL`、三角形最大屏幕尺寸的取值，用阶段 0 统计决定。
- T 型连接处的漏像素是否可见，决定是否切换到"边顶点集合只由 F 决定"的方案。
- 逐片元求导的性能，必要时退化为顶点法线插值加阈值控制。
