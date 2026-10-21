# 参数曲面 GPU 裁剪计划

接在 [plan.md](plan.md) 的阶段 2 之后，调研见 [trim-research.md](trim-research.md)。目标是逐片元裁剪：
在参数域里判断片元 `(u, v)` 是否在裁剪环内。点分类用 Slug（Lengyel 2017）的算法：裁剪曲线统一近似
成二次 Bézier，射线与二次曲线求闭式根，用根判定码保证共享端点不重复计数。裁剪曲线数据的访问
（band、格子、树）抽象成可替换的加速结构，第一个实现是均匀 band 加预分类格子。

## 目标与范围

目标：带裁剪的 patch 在屏幕上只显示裁剪环内的部分，边界逐像素精确（误差为二次近似容差加细分误差），
同时用叶子级分类把完全在环外的细分剔除掉，完全在环内的叶子不付片元裁剪的开销。

本阶段做：

- CPU 预处理：裁剪曲线转二次，构建加速结构。
- 片元裁剪（二值判定，discard）。
- 细分阶段的叶子分类（外 / 内 / 部分），按是否裁剪分桶。
- 基于距离的外扩（遮住跨 face 裂缝）。

本阶段不做：

- 覆盖率抗锯齿，设计记录在"后续：覆盖率抗锯齿"一节。
- 跨 face 的几何 watertight（trim 边走"边细分"逻辑，见 plan.md），这里只用外扩掩盖。
- 按裁剪边加密细分（部分覆盖叶子里环外的面积仍然被光栅化）。
- 视角相关的二次近似。

## 输入数据

steam-controller，带裁剪的 patch 9969 个（统计脚本不在仓库里，结论记在这里）：

| 项 | 数据 |
| --- | --- |
| 曲线 | 直线 31067（其中 `PatchBoundary` 23072，`Pole` 71），非有理三次 10277，有理二次 122（圆锥弧） |
| 每 patch 曲线数 | p50 4，p90 4，p99 8，max 138 |
| 每 patch 环数 | p50 1，p90 1，max 4 |
| 二次近似（uv 容差 1e-3） | 共 4.7 万段，每 patch p50 4 / p99 11 / max 146 |
| 二次近似（uv 容差 1e-4） | 共 5.9 万段，每 patch p50 4 / p99 19 / max 168 |
| 二次近似（uv 容差 1e-5） | 共 8.9 万段，每 patch p50 6 / p99 37 / max 234 |
| 8 条水平 band 中最满的一条（容差 1e-4） | p50 3，p90 6，p99 11，max 60 |

结论：

- 绝大部分 patch 只有几条曲线，关键是让大部分片元不走求交（预分类），而不是复杂的加速结构。
- 长尾 patch 的曲线集中在很窄的 v 范围内（8 条 band 时最满的仍有 60 段），均匀 band 对它们效果差，
  这是把加速结构抽象成可替换实现的主要原因。
- 静态的紧容差就够用：1e-5 时总数约 9 万段，每段 3 个 f32 vec2，约 2 MB。不需要视角相关。
- 数据保证环内相邻曲线首尾逐位相等（`TrimLoop` 文档），这是根判定码无 epsilon 正确的前提。
- 环方向：外环逆时针、孔顺时针、材料在左边，所以用 nonzero winding。

## 总体架构

```text
[CPU 预处理, 每个模型一次]
  每个带裁剪的 patch: 曲线 -> 二次段(共享端点只求值一次) -> 构建加速结构
  上传: 段池 + patch 裁剪头(段区间, 加速结构偏移) + 加速结构的 u32 表
[每帧细分, plan.md 的管线]
  L.b 内部: 工作项的 uv 矩形用加速结构的矩形查询分类
              全外 -> 剔除(整棵子树)   全内 -> 继续, 不需要裁剪   部分 -> 继续, 需要裁剪
  L.d 决策: 叶子带上 "部分覆盖" 标志, 按 (k, 是否裁剪) 分 10 个桶
[绘制]
  vertex: 由叶子 key 精确算出格点 uv, 输出 uv(透视插值) 和 patch 裁剪头下标(flat)
  fragment(仅裁剪桶): 加速结构给出 (基础 winding, 候选段), 内核累加 winding
                      winding == 0 -> discard; T4 起再加距离做外扩
```

## 关键设计

### 曲线转二次

每个 `TrimCurve` 转成若干条非有理二次段 `(p0, p1, p2)`，环内顺序和方向不变：

- 直线：`p1 = (p0 + p2) / 2`。shader 里 `a = p0 - 2p1 + p2` 接近 0 时走线性分支（Slug 的做法）。
- 非有理三次：均匀拆成 n 段，每段用 `p1 = (3(P1 + P2) - P0 - P3) / 4`。整条三次的误差上界是
  `√3/36 · |P3 - 3P2 + 3P1 - P0|`，拆成 n 段后除以 n³，所以 n 有闭式解，不用采样。
- 有理二次（只有 122 条）：递归二分，每段用端点加中点插值的二次，稠密采样检查误差。
- `Pole` 段（首尾是同一点）直接丢掉，环仍然闭合。
- 拆分点用 f64 de Casteljau 在原曲线上只求值一次，相邻两段共用这个值，再统一转 f32。这样二次段
  之间、原曲线之间的端点在 f32 里都逐位相等。

容差：patch 的 3D 容差除以 `max |∂S/∂u|, |∂S/∂v|` 的控制网上界（`n·max|Δb|`），换算成 uv 容差，
下限取 f32 在 [0, 1] 上能分辨的量级（约 1e-6）。3D 容差的默认值待定（见"未决问题"），先用
`model.tolerance` 的量级。

### 点分类内核（Slug 的根判定码）

二次段的控制点先减去采样点 `(u, v)`。对朝 +u 的水平射线：

- 根判定码：`shift = (y0 > 0 ? 2 : 0) + (y1 > 0 ? 4 : 0) + (y2 > 0 ? 8 : 0)`，
  `code = (0x2E74 >> shift) & 3`。bit 0 表示根 t1 计入，bit 1 表示根 t2 计入。code 只由三个 y 的符号
  决定，曲线首尾相接处、水平切点处都不会重复或漏计，不需要 epsilon。
- 根：`a = p0 - 2p1 + p2`，`b = p0 - p1`，`t1,2 = (b.y ∓ sqrt(max(b.y² - a.y·p0.y, 0))) / a.y`；
  `|a.y|` 很小时 `t = p0.y / (2·b.y)`。
- 计入的根 x 坐标 > 0 时，t1 贡献 +1、t2 贡献 -1（符号约定在 T0 用测试确认）。
- 片元在材料内当且仅当 winding ≠ 0。

`PatchBoundary` 直线保留，它们是闭合环的一部分。+u 射线只可能穿过 u = 1 上的边界段，水平的边界段
被判定码直接跳过，开销很小。

边界情况：透视插值得到的 uv 可能略超出 [0, 1]。点在 u ≥ 1 时射线漏掉 u = 1 的边界段，导致 cell
边上出现漏像素。所以分类前把 uv 夹到 `[0, 1 - 2⁻²⁰]²` 内，并为这种情况写测试。

### 裁剪数据访问的抽象

band、格子、kd-tree 本质上都是同一件事：告诉内核"这个点只需要看哪些段，其余段的贡献是多少"。
所以把它抽象成加速结构，和点分类内核分开。

点查询的约定：给定 patch 和点 p，加速结构返回基础 winding `w0` 和一组候选段，满足

```text
winding(p) = w0 + Σ_{s ∈ 候选} crossing(s, p)
```

其中 `crossing` 是上一节的 +u 射线带符号交点数。各种结构都是这个约定的特例：

| 实现 | `w0` | 候选段 |
| --- | --- | --- |
| 全部段（参考实现） | 0 | patch 的全部段 |
| 均匀 band | 0 | 所在 band 的段，按 max u 降序，可提前退出 |
| 预分类格子 | 常数格直接给出 | 常数格为空，否则交给下一层结构 |
| RAVG 式格子 | 格子的 winding 偏移 | 裁剪到格子内的段 + 右边界辅助段 |
| kd-tree（Schollmeyer 2019） | 叶子预存的交点数 | 与叶子重叠的段 |

其余约定：

- **矩形查询**（叶子分类用）：给定 uv 矩形 R，返回外 / 内 / 部分。可以保守，"部分"总是合法的答案。
  通用实现是：取 R 的候选段，与 R 不相交时用一次点查询判断中心。各实现可以给出更快的版本，比如
  R 覆盖的预分类格子全部同一类时直接返回。
- **距离**（T4 外扩和以后的覆盖率用）：要求候选段包含所有离 p 不超过 r 的真实段，r 是结构声明的
  外扩半径（band 和格子按 r 外扩后再收集段）。结构生成的辅助段（如 RAVG 的右边界竖线）只参与
  winding，不参与距离，用段上的一个标志位区分。
- **提前退出属于结构内部**：结构驱动循环，把候选段逐个交给内核的回调。像 band 的"max u 已在 p 左边"
  这类停止条件由结构自己检查。

实现形态：

- CPU：`TrimAccelBuilder` 接收一个 patch 的二次段，输出 `u32` 字（追加到共享的 u32 表）。辅助段追加到
  共享的段池。CPU 侧另有一份与 shader 逐操作对应的 f32 查询实现，用于测试。
- shader：`ShaderTrimData`（`src/tessellation/trim.rs`）按 patch 裁剪头里的 `kind` 在运行时
  `switch`，点查询、矩形查询和距离查询各有一份与 CPU 镜像逐操作对应的实现。实现时没有做成按结构区分
  pipeline 的 trait：`kind` 本来就存在每个 patch 的头里，运行时分支让同一个模型可以按 patch 混用结构
  （例如简单 patch 用 band、长尾 patch 用格子），也不会让 pipeline 数翻倍。
- 测试：每种实现都和"全部段"参考实现比对。段集合相同时 winding 必须逐位一致；会裁剪段的实现
  （RAVG 格子），只允许在离曲线很近的点上不一致。

### 第一个实现：band 加预分类格子

选它是因为预处理最简单、没有专利问题（Slug 已进入公有领域），并且对占绝大多数的简单 patch 已经够用。

- 预分类格子：每个 patch 一张 `n×n` 的格子，n 是 2 的幂（1 到 16，按段数自适应），每格 2 bit：全内、
  全外、不确定。格子与所有段的控制点包围盒都不相交时，用参考实现判断格子中心得到常数类，否则为
  不确定。n 取 2 的幂时，`floor(uv·n)` 在 f32 里是精确的。
- 水平 band：参数域在 v 方向均匀切成 `nb` 条，按曲线数自适应，取最小的 2 的幂，使最满的 band 不超过
  约 4 段，上限 16。段按控制点包围盒的 v 范围进入跨过的 band，水平段（三个 y 相等）不进。band 内按
  `max u` 降序排序，`max u < p.u` 时退出。
- 点查询：先查预分类格子，常数格直接返回；否则扫 band。
- 矩形查询：R 覆盖的预分类格子都是同一个常数类时直接返回，否则扫 R 的 v 范围覆盖到的 band，做
  包围盒相交测试。
- Slug 早期的 band split（每条 band 存两个方向的列表）已被作者删掉，原因是分支发散，我们也不做。

备选实现（需要时再做，通过抽象层替换）：

- RAVG 式格子（Nehab & Hoppe 2008）：片元只扫自己格子内的段，开销与局部复杂度成正比，长尾 patch 上
  有上界，而且同时充当预分类。缺点是预处理要把二次段裁剪到格子，并生成辅助段；另有美国专利
  US7872648（推定到 2029-11，见 trim-research.md），可以做成可选模块。
- 按端点切的不均匀条（Schollmeyer 2009 的 double binary partition），专门针对 v 方向密集的长尾。

### 叶子分类

在 L.b 里对每个工作项的 uv 矩形 R 做矩形查询（plan.md 的二进位 key 直接给出 R）：

- 外：剔除整棵子树。剔除不影响 watertight，因为边吸附只依赖边自己，不依赖邻居。
- 内：继续，并标记为不需要裁剪。
- 部分：继续，标记为需要裁剪。
- 外扩（T4）时剔除要保守：R 先在 uv 里外扩 δ 像素对应的量。这个量由 L.b 已有的投影尺寸估计：
  R 的 uv 尺寸除以屏幕尺寸，乘 δ，再按浮点位向上取整到 2 的幂（`dilation_margin`），这样 GPU 和
  CPU 参考逐位一致，外扩后的矩形在 f32 里也是精确的。投影尺寸取包围盒对角线，某些方向上偏大，所以
  外扩是近似的。
- 不在工作项 key 里记录"父节点全内"，每层重新分类。全内的父节点下，子节点的分类结果必然也是全内，
  每层只多一次查询。
- 叶子的 `LEAF_FLAG_TRIMMED` 改为"部分覆盖"的含义。"patch 是否带裁剪"仍在 item 的 flag 里，供
  可视化开关使用。
- 分桶从 5 个变为 10 个：`(k, 是否裁剪)`。不裁剪的桶用不带 discard 的 pipeline，保留 early-Z。
  `bucket_leaves` 的容量随之加倍。

L.b 现在已经接近每个 stage 10 个 storage buffer 的上限，所以裁剪数据打包成两个 buffer：段池
（`vec4` 数组，每段两个 vec4：`p0 p1` 和 `p2 max_u flags`）和一个 `u32` 表（patch 裁剪头加上各结构自己
的数据）。

CPU 参考（`parametric-tessellation`）做同样的分类，叶子集合比对继续成立。分类的不一致只允许出现在
曲线到 R 的距离处在 f32 舍入量级的情况下。

### 片元接口

- `TessLeafDraw` 加上子 patch key（或者 uv 矩形）。vertex shader 由 key 和格点下标算出格点 uv，
  二进位运算在 f32 里是精确的。边吸附后的顶点位置不是 `S(uv)`，但偏差在边容差以内，与细分误差
  同阶，可以接受。
- vertex shader 读 item 表得到 patch，再读 patch 裁剪头，把头的下标作为 flat varying 传出，`uv` 用透视
  插值传出。fragment 只读两个裁剪 buffer。
- 逐像素法线（plan.md 的后续项）同样需要这个 uv，两者共用。

### 距离与外扩（T4）

跨 face 裂缝：相邻两个 face 各自用自己的 pcurve 裁剪，各自的曲面又带细分误差，共享边处会有像素级
的缝或重叠。不透明渲染下重叠无害，所以把裁剪区域外扩约 δ ∈ [0.5, 1] px。

- 距离：遍历候选段时，对每条真实段计算近似最近点（RAVG 的隐式形式加参数反演，见 trim-research.md），
  得到 uv 里的距离向量，取最短的。和 winding 在同一次遍历里完成，不需要垂直 band。
- 换成像素：用 `dpdx(uv)` / `dpdy(uv)` 组成 2×2 矩阵 J（像素到 uv），距离向量 v 对应的像素长度是
  `|J⁻¹ v|`。参数映射经常各向异性且有斜切，不能像 Slug 那样用 `fwidth` 按轴分开。
- 判定：winding ≠ 0，或像素距离 < δ 时保留片元。
- 候选段（实现时修正）：没有用预处理时固定的上限 r。远景下一个像素可以占 patch 参数域的百分之几，
  固定 r 要么截断外扩，要么让预分类格子几乎全是不确定格。改为片元按 J 算出 δ 像素圆在参数域里的
  包围盒半宽 `e = (|∂uv/∂x| + |∂uv/∂y|)·δ`，band 结构只访问与 `[v − e_v, v + e_v]` 相交的 band，跳过
  包围盒不在 `uv ± e` 内的段，遇到 `max_u < u − e_u` 时停止（`nearest_vector_within`）。band 包含所有
  包围盒与其 v 范围相交的段，所以这样是保守的。`BandGrid::distance_radius` 为 0，表示结构本身不保证
  任何半径，由查询给出范围。只有中心在环外的片元才计算距离。
- 副作用：凸棱处轮廓最多外扩 δ 像素；G1 连续的边（大部分倒角）在重叠区深度几乎相同，会有
  z-fighting，但两侧着色相同，预期看不出来，需要实测。
- 先用 δ = 0 截图记录裂缝的实际表现，再决定 δ 的默认值，或者是否需要外扩。结果见"T4 结果"，
  默认值取 0.5 px。

## 后续：覆盖率抗锯齿

本阶段不做。裁剪边先靠 TAA 抗锯齿；在 MSAA 下，discard 是逐像素的，裁剪边没有抗锯齿。T4 的距离
做完后，覆盖率只差最后一步。设计先记录如下：

- 覆盖率由像素单位的有符号距离 d 得到：最简单是 `saturate(0.5 + d)`；更准确的是半平面与像素盒的
  闭式面积，或者 Schollmeyer 2019 的二维预积分表（按距离和法线角查表）。
- 符号由 winding 决定，距离由 T4 的近似最近点给出。法线方向用 J⁻¹ 变换后的距离向量方向。
- 输出：MSAA 下走 alpha-to-coverage；不开 MSAA 时阈值取 0.5，等价于二值判定。不做 alpha 混合，因为
  曲面是不透明的，需要写深度。
- 加速结构的候选段要覆盖整个像素足迹，r 至少要大于像素对角线对应的 uv 长度（RAVG 用 10～20% 的
  扩展格子）。足迹超过 r 时退化为二值判定。
- 参考：Slug 用水平、垂直两条射线的覆盖率按根的距离加权混合，适合字形这种轴对齐的情况；Schollmeyer
  用切线距离加预积分表；RAVG 用近似最近点距离加 J 的预滤波，并在一次遍历里支持多采样点超采样。
- 开始的条件：需要 MSAA 下的质量，或者 TAA 在裁剪边上出现可见的闪烁。

## 实施阶段

### T0：CPU 预处理、抽象与参考实现

位置：`parametric-tessellation` 新增 `trim` 模块（只依赖 `parametric-normalized-model`）。

- 曲线转二次；点分类内核，包括 f64 版本和逐操作模仿 shader 的 f32 版本。
- 加速结构的 CPU 接口，以及"全部段"参考实现和"band 加预分类格子"实现。
- 参考真值：原始有理曲线用很细的折线（1e-8）计算 f64 winding。
- 测试：
  - 随机采样点，以及专门构造的难例（射线正好穿过曲线端点的 v、水平切点、u = 1 边界、uv 越界）。
  - 二次段与真值比对，只允许距离曲线在二次近似容差内的点不一致。
  - f32 内核在难例上不一致数为 0；band 实现与"全部段"实现逐位一致；矩形查询保守。
- 统计：steam 的二次段数、band 数和预分类格子分辨率的分布、常数格比例、最满 band。CLI 的 SVG 裁剪图
  可以叠加二次段、band 和格子，供肉眼检查。

#### T0 结果

实现在 `parametric-tessellation` 的 `trim` 模块（`src/trim`，模块文档里有编码布局的总览）。

API：

- 转二次：`TrimQuadraticConfig::for_model(model)`（3D 容差取 `model.tolerance`，换算后的 uv 容差夹到
  `[1e-6, 1e-3]`），`trim_quadratics(patch, config) -> PatchTrimQuadratics`（段、环区间、uv 容差、
  失败数），`parametric_speed_bound`，`cubic_piece_count`。
- 内核：`crossing(&EncodedSegment, uv)`（f32，逐操作对应 shader），`crossing_f64(&TrimQuadratic, uv)`，
  `root_code`，`clamp_uv`。
- 加速结构：trait `TrimAccel`（`kind`、`build`、`point_query`、`rect_query`、`distance_radius`），实现
  `AllSegments`、`BandGrid`；查询看到的数据是 `PatchTrimView`，按存储的类型取实现用 `accel_for_kind`。
- 编码：`encode_trims(model, config, &dyn TrimAccel) -> TrimEncoding { segments, table, stats }`，
  `TrimEncoding::{header, patch_view, winding, is_inside, classify_rect}` 是 GPU 解码路径的 CPU 镜像。
- 真值：`TrimReference`（原始有理曲线的细折线，f64 winding 和距离）。

T1 要解码的布局：

- 段池：每段两个 vec4，`(p0.x, p0.y, p1.x, p1.y)`、`(p2.x, p2.y, max_u, bitcast<f32>(flags))`，
  `flags` 的 bit 0 是辅助段（目前没有结构生成辅助段）。一个 patch 的段连续存放，按环排列，相邻段端点
  逐位相等。
- u32 表：开头是每个 patch（模型里的全部 patch，按 patch 下标）4 个字的头
  `segment_offset, segment_count, accel_offset, kind`。`kind`：`0` 全内（未裁剪，或裁剪覆盖整个参数域），
  `1` 全外，`2` 全部段（无数据），`3` band 加预分类格子。
- `kind = 3` 的数据（从 `accel_offset` 起）：
  - 字 0：`band_log2 | grid_log2 << 8`。
  - 接着 `ceil(n² / 16)` 个字的预分类格子（`n = 1 << grid_log2`）：格子 `cy·n + cx` 在第 `cell / 16`
    个字的 `2·(cell % 16)` 位起，`0` 不确定、`1` 外、`2` 内。样本所在格子是 `floor(uv·n)`。
  - 接着 `nb = 1 << band_log2` 个 band 字：`start | count << 16`，`start` 相对于 band 字之后的列表起点。
    样本所在 band 是 `floor(v·nb)`。
  - 最后是各 band 的列表：patch 内的局部段下标，每个一个 u32，按 `max_u` 降序。
- 片元：`uv` 夹取后读头；`kind` 为 0 / 1 直接给出结果；`kind = 3` 先查格子，不确定时扫样本所在的 band，
  遇到 `max_u < u` 停止，对每段累加 `crossing`，winding ≠ 0 即在材料内。

相对计划的修正：

- 根判定码照 Slug 参考实现取符号位（负为 1）：`shift = neg(y0) | neg(y1) << 1 | neg(y2) << 2`，
  `code = (0x2E74 >> shift) & 0x0101`，bit 0 对应 t1，bit 8 对应 t2。"点分类内核"一节写的 `y > 0` 形式
  不等价（+0 算作非负）。
- 符号：t1 贡献 −1，t2 贡献 +1（与 Slug 的 `xcov` 相反），逆时针外环内部 winding 为 +1。测试已确认与
  本数据的环方向一致。
- 求根不用 Slug 的 `|a| < 1/65536` 线性分支，因为参数域里的短段的 `a` 经常低于这个固定阈值。改用无抵消
  形式：`b ≥ 0` 时 `t1 = c / (b + d)`，`b ≤ 0` 时 `t2 = c / (b − d)`，其余用 `(b ∓ d) / a`。判别式 ≤ 0 时两个
  根都取 `b / a`（与 Slug 相同）。这一点不能省：控制点越过射线而曲线没到达时，两个根都被判定码选中，
  只有取同一个 t 才能相互抵消。steam 第一次验证时就是这个错误。
- 内核的定义包含"`max_u < u` 的段贡献 0"，这样 band 的提前退出与扫描全部段逐位一致。
- 夹取范围改为 `[2⁻²⁰, 1 − 2⁻²⁰]²`。根判定码把正好落在段端点 v 上的射线视为从下方经过，所以 v = 0 的
  样本会漏掉从 v = 0 开始的边界段，整条底边都被判为外。
- 位于参数域边上的段（三个控制点的 u 或 v 全为 0 或全为 1）不参与包围盒测试，因为样本到不了那里，
  但它们仍参与 winding。否则所有贴边的格子都是不确定，覆盖整个参数域的裁剪也无法降级为全内。
- `Pole` 曲线不能丢：它只在 3D 里退化，在参数域里是环上的一段边界。只丢参数域里退化成点的曲线。
- 水平段也进 band，矩形查询需要它们，点查询的代价只是一次判定码。
- 预分类格子固定为 16×16（配置项 `grid_log2`），没有按段数自适应：每个 patch 16 个字，内存可以忽略。
- 新增 `kind` 0 / 1：未裁剪的 patch，以及裁剪后整个参数域都是材料或都不是材料的 patch，没有段也没有
  结构数据。
- 抽象只有一个 trait `TrimAccel`，构建和 CPU 侧的查询都在其中，没有单独的 `TrimAccelBuilder`。

steam-controller（`RENDIATION_TEST_PNM=… cargo test --release -p parametric-tessellation steam --
--ignored --nocapture`）：

| 项 | 数据 |
| --- | --- |
| 模型容差 | 0.0099 |
| patch / 带裁剪 | 17387 / 9969 |
| 降级为全内 / 全外 | 2 / 2 |
| uv 容差 | min 1.5e-4，p50 1e-3（上限），max 1e-3 |
| 二次段 | 46123，每 patch p50 4 / p90 6 / p99 11 / max 168，近似失败 0 |
| band 数（1 / 2 / 4 / 8 / 16） | 6879 / 1974 / 361 / 120 / 631 |
| 最满 band | p50 4，p90 4，p99 6，max 46 |
| 预分类格子 | 不确定 13.1%，外 26.6%，内 60.2% |
| 预处理（release） | band 加格子 32 ms，全部段 5 ms |
| 上传 | u32 表 1285 KiB，段池 1441 KiB |
| 验证 | 76.7 万个点：band 扫描与全部段 0 不一致；其中 35 万个离曲线超过 2 倍容差的点与真值 0 不一致；随机二进位矩形的分类 0 错误 |

遗留：

- 默认 3D 容差下，多数 patch 的 uv 容差被上限 1e-3 截住。是否合适，留给 T4 的截图判断。
- f32 内核与 GPU 的结果能否逐位一致，取决于编译器是否合并 FMA，T1 的对比测试会暴露。
- 长尾：最满 band 最多 46 段，631 个 patch 用满了 16 条 band。
- 距离约定只有接口（`distance_radius`），没有实现。CLI 的 SVG 叠加图没做。

### T1：GPU 数据与 shader 分类函数

- 上传段池和 u32 表，读回后与 CPU 逐项比对。
- `TrimAccelShader` 和内核放进 `shader_lib`，点查询和矩形查询都与 CPU f32 版本做随机对比测试，
  难例必须完全一致。

#### T1 结果

实现在 `rendiation-parametric-rendering` 的 `src/tessellation/trim.rs`，测试在 `tests_trim.rs`：

- `TrimDeviceData`：`TrimEncoding` 的段池（`TrimSegmentGpu`，两个 vec4）和 u32 表，两个只读 storage
  buffer。`encode_model_trims` 是默认编码（模型容差、`BandGrid::DEFAULT`）。
- `ShaderTrimData`：`winding`、`classify_rect`（结果为 `TRIM_RECT_*`）、`nearest_vector` /
  `nearest_vector_within`（T4），内核 `shader_trim_crossing` / `shader_root_code`。
- 合成场景从 `parametric-tessellation` 的测试里移到公开的 `test_scenes::trim`（`trim_scenes`、
  `trim_config`、`sample_points`、`trimmed_random_model`），GPU 测试和 viewer 测试共用。

GPU 与 CPU f32 镜像的对比（Apple M5 Pro）：

| 用例 | 点查询 | 矩形查询 |
| --- | --- | --- |
| 合成场景，band 加格子 / 全部段 | 各约 3.2 万点，18 个不一致，全部正好在段端点上 | 各 1.5 万个矩形，0 不一致 |
| steam，全部带裁剪 patch | 68.8 万点，600 个不一致，到段的距离都不超过 2⁻²⁰ | 7.97 万个矩形，0 不一致 |

不一致的原因：采样点正好在两段的公共端点上（相对坐标 `p2 = (0, 0)`）时，根的 x 是舍入噪声，它的符号
取决于 FMA 合并和除法精度（Metal 默认 fast math）。点在裁剪边界上时两种结果都合理，所以测试只要求
不一致的点到段的距离 < 1e-6，矩形查询要求完全一致。

### T2：片元裁剪

- `TessLeafDraw` 带 key，vertex 输出 uv 和裁剪头下标，裁剪 pipeline 在 fragment 里 discard。
  这一步先不分桶：带裁剪 patch 的所有叶子都走裁剪 pipeline。
- viewer：带裁剪 patch 的显示方式在"正常 / 调淡 / 隐藏"之外新增"裁剪"，并设为默认。
- 离屏测试：GPU 裁剪渲染的覆盖率与 CLI 的 CPU 裁剪三角化（`mesh.rs`）渲染结果对比，差异只能出现在
  边界像素上。

#### 管线改造结果

与裁剪数据无关的部分已先做完（T2 和 T3 的绘制侧接口），裁剪数据本身还没有接入：

- `TessLeafDraw` 变为 8 个 u32：`vertex_base, k, item, flags, level, ix, iy, patch`。`(level, ix, iy)` 是
  子 patch key，`patch` 是 `ParametricNormalizedModel::patches` 的下标（由 L.d 从 item 表读出，CPU
  参考在 `from_cpu_reference` 里填）。原来单独的 `leaf_keys` buffer 删掉了，顶点 pass 直接从叶子记录
  读 key，每个顶点 pass 少绑定一个 storage buffer。
- 分桶：`BUCKET_COUNT = 10`，`bucket_of(log2 k, trimmed) = log2 k + 5·trimmed`，配套 `bucket_k` /
  `bucket_is_trimmed`。`K_BUCKET_COUNT` 改名为 `K_RESOLUTION_COUNT`（k 的个数，仍为 5）。"是否裁剪"
  目前是 `LEAF_FLAG_TRIMMED` 的原语义（叶子所属 patch 带裁剪），T3 改成"部分覆盖"时只需改 L.d 和
  `from_cpu_reference` 里这一个标志。计数器布局随之调整（`COUNTER_BUCKETS` 10 项、
  `COUNTER_VERTEX_PREFIX` 11 项，其后各段后移，`COUNTER_COUNT` = 104）。`bucket_leaves` 容量为
  `leaf_capacity × 10`，`TessCapacity::limited` 增加了叶子记录和桶列表的单 binding 大小限制。
  统计增加 `trimmed_leaves`，`leaves_per_k` 仍按 k 合计两类桶。
- 绘制 vertex：由 key 和格点下标算出定点参数 `(ix·k + i)·2^(PARAM_BITS − level) / k`，再乘 `2^-PARAM_BITS`
  得到 f32 uv，与 CPU `param_to_f64` 一致且精确。输出 `TessellationParameter`（透视插值）、
  `TessellationItemIndex` 和 `TessellationPatchIndex`（整数输出，flat）。新增颜色模式 `Parameter`
  （uv 作为红绿），方便在 viewer 里检查。
- 裁剪钩子：`render_tessellation_output(ctx, output, uniform, trim, before, after)` 新增
  `trim: Option<&dyn TessellationTrimFragment>`，只用于带 `LEAF_FLAG_TRIMMED` 的桶。trait 方法
  `build_trim(builder, binding, TessellationFragmentInput { uv, item, patch })` 在着色之后运行，可以
  discard 或覆盖 `DefaultDisplay`；它的资源（T2 的段池和 u32 表）在 `build_trim` 里绑定，并在
  `setup_pass` 里按同样顺序绑定，排在桶绘制的四个资源之后；pipeline hash 包含它的
  `hash_pipeline_with_type_info`。没有传 `trim` 时裁剪桶和普通桶只差 pipeline 变体（桶号）。
- storage buffer：绘制 pipeline 现在用 3 个（顶点、叶子、桶列表），T2 再加段池和 u32 表共 5 个，
  在 WebGPU 默认每 stage 8 个的限制内。
- viewer：`TrimmedPatchStyle` 新增 `Trimmed`（界面里为"Trimmed"，悬停提示说明尚未裁剪），默认值
  仍是 `WashedOut`；viewer 目前传 `trim = None`，`Trimmed` 的效果等同 `Normal`。T2 接入时由 viewer
  在 `Trimmed` 风格下传入裁剪 fragment，并把默认改为 `Trimmed`。

验证：

- 新测试 `tests_draw::trimmed_fragment_receives_exact_parameter`：随机模型里每隔一个 patch 标记为带
  裁剪，用测试钩子把 `(u, v, item, patch + 2)` 写进 `Rgba32Float` 目标。对每个裁剪桶的像素，用 CPU 在
  `(u, v)` 处求曲面点并投影，到像素中心的最大距离 0.22 px（阈值 `pixel_error + 0.05`），item 和 patch
  与 CPU 的 item 表一致；普通桶的像素不经过钩子。
- 原有测试全部通过：crate 内 24 个测试；steam 的 GPU / CPU 对比（ignored 测试）各视角叶子数、
  不一致叶子数、watertight 与改动前一致（例如 fit 17471 / 17471，不一致 14，共享顶点不一致 0）；
  viewer 离屏测试（合成模型和 steam，MSAA / TAA / 隐藏裁剪 patch / CPU 参考）覆盖率不变，steam
  17387 叶子、29.7 万三角形。

#### T2 结果

- `TrimFragment`（`TessellationTrimFragment` 的实现）：winding 为 0 时 discard（T4 加上外扩）。
  `GpuTessellator::with_trims` 接收已上传的 `TrimDeviceData`，`new` 自己编码。
- viewer：`ParametricModelSource::trims`（`Arc<TrimEncoding>`，在加载线程上编码，steam 约 32 ms），
  第一帧上传为 `TrimDeviceData`，GPU 生产者、CPU 参考和绘制共用；`TrimmedPatchStyle::Trimmed`
  设为默认，界面说明已更新。
- 验证方式的修正：覆盖率对比没有用 CLI 的 `mesh.rs` 三角化，而是直接用原始有理曲线的 f64 winding
  逐像素对比（片元的 uv 已知），比三角化更直接，也更严格。

测试：

- `trim_fragment_matches_cpu`：10 个互不重叠、带裁剪的 patch，800×600。每个裁剪片元的 GPU winding
  与 CPU f32 镜像 0 不一致；discard 的结果与原始曲线的 winding 0 不一致（测试允许边界像素不同，
  实际为 0）。
- viewer 离屏测试：原来合成模型的"裁剪"只是一条不闭合的直线，编码后整个 patch 判为环外；改为参数域
  边界加一个圆孔，并新增 `whole_cells` 用例。覆盖率为隐藏 6.3% < 裁剪 14.0% < 整个 cell 17.0%，
  GPU 与 CPU 参考一致。steam（640×480）为隐藏 6.1%、裁剪 13.4%、整个 cell 13.6%，渲染图中整个
  cell 时伸出的鳍片在裁剪后消失。

### T3：叶子分类

- L.b 分类与剔除，L.d 输出部分覆盖标志，10 个桶。CPU 参考同步实现，叶子集合比对。
- 统计读回：全外剔除的工作项数、全内叶子数、部分覆盖叶子数。
- 测量：叶子数、三角形数和帧时间的变化，以及裁剪 pipeline 覆盖的片元比例。

#### T3 结果

- GPU：L.b 在视锥剔除之后用 `classify_rect` 分类工作项的矩形（T4 起先外扩）。状态字增加
  `STATUS_TRIM_CULL`（代码 5）、`STATUS_TRIMMED_BIT`（部分覆盖）和 `STATUS_TRIM_INSIDE_BIT`，读状态
  码一律通过 `status_code` 屏蔽这些位。L.d 用这些位设置 `LEAF_FLAG_TRIMMED`，并累加
  `COUNTER_TRIM_CULLED` / `COUNTER_TRIM_INSIDE_LEAVES`。统计新增 `trim_culled`、`trim_inside_leaves`。
- CPU 参考：`Tessellator::with_trims`、`Leaf::trimmed`、统计 `trim_culled` / `trim_inside_leaves` /
  `trim_partial_leaves`；`compare_leaf_sets` 同时比较 `trimmed` 标志。
- 决策：分类可以关闭（`GpuTessellator::set_trim_classification`，默认开）。viewer 只在 `Trimmed` 风格
  下开启，其他风格仍画整个 cell，"调淡 / 隐藏"这些调试视图继续有意义。关闭时 `LEAF_FLAG_TRIMMED`
  保持原语义（叶子所属 patch 带裁剪）。
- 测试：CPU 测试 `trim_classification_keeps_the_material`（不需裁剪的叶子在环内，被剔除的区域在环外，
  保留下来的 key 的决策不变）；GPU 与 CPU 在带真实裁剪的合成场景上叶子集合（含标志）完全一致。

steam，1920×1080，0.5 px，分类关 → 开（叶子与三角形数 GPU 与 CPU 一致，不一致叶子同改动前）：

| 视角 | 叶子 | 三角形 | 裁剪叶子 | 剔除 / 全内 | 细分帧时间 | 片元总数 | 走裁剪 pipeline 的片元 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| fit | 17471 → 17468 | 716k → 716k | 10053 → 10019 | 3 / 29 | 5.3 ms 不变 | 80.3 万不变 | 82.8% → 82.4% |
| fit:4 | 10875 → 10432 | 1.50M → 1.29M | 6823 → 6252 | 294 / 128 | 7.84 → 6.55 ms | 691 万 → 543 万 | 92.9% → 73.4% |
| fit:20 | 3635 → 3525 | 435k → 381k | 1889 → 1628 | 110 / 151 | 4.0 ms 不变 | 312 万 → 294 万 | 88.9% → 38.0% |
| fit:80 | 3045 → 2939 | 453k → 400k | 1628 → 1321 | 82 / 201 | 4.0 ms 不变 | | |

片元数用加法混合计数（没有深度测试，含重叠）。fit 视角下效果很小：大部分 patch 在第 0 层就是叶子，
根矩形是整个参数域，几乎总是部分覆盖。这时约 38% 的裁剪片元被 discard。

### T4：距离与外扩

- 近似最近点距离、基于雅可比的像素换算、可配置的 δ、按 r 收集候选段、L.b 的保守剔除。
- 先用 δ = 0 截图记录跨 face 裂缝，再看外扩后裂缝是否消失、凸棱处轮廓的变化，以及 G1 边上
  z-fighting 是否可见。

#### T4 结果

- 近似最近点：`nearest_point`（CPU f32 镜像）和 `shader_nearest_point`，即 RAVG 的隐式形式加参数反演。
  直线在编码时按 f64 共线性加 `SEGMENT_FLAG_LINE`，近似共线（`|4·面积| ≤ 1e-5·弦长²`，
  `NEAR_LINE_RATIO`）也取弦，弦与曲线的偏差小于弦长的 2.5e-6。
  - 精度：合成场景中离曲线 0.02 以内的 10 万个点，与原始曲线折线距离相比，相对误差最大 0.9%（距离
    > 1e-3 时）。GPU 与 CPU 的距离绝对误差最大 4.9e-7。
- 片元：中心在环外且 δ > 0 时，用 `nearest_vector_within` 求距离向量，经 J⁻¹（`uv_to_pixels`）换算
  成像素，长度 < δ 时保留。`TrimFragment::new(device, data, δ)` / `set_dilation` 通过 uniform 设置 δ。
- 分类：`TessUniform::trim_dilation`，L.b 和 CPU 参考按 `dilation_margin` 外扩矩形。`dilation_margin`
  在 GPU 与 CPU 上 2 万个随机输入逐位一致。外扩 0 / 1.5 / 200 px 时，GPU 与 CPU 的叶子集合都一致，
  合成场景剔除的工作项数为 12 / 12 / 0。
- `trim_dilation_keeps_near_fragments`：外扩 2.5 px 时新增 5432 个片元，全部在环内像素 4 px 以内，
  原来保留的片元没有一个丢失。
- 默认值：viewer 的 `DEFAULT_TRIM_DILATION` = 0.5 px，界面里可调；库里 `GpuTessellator` 默认 0。

裂缝评估（`steam_trim_cracks`，1920×1080，深度测试）。间隙像素定义为：在一行或一列上，比相距 1 或 2
像素的两侧都远（超过 2%，或者是背景）的像素。

| 视角 | δ | 间隙像素 | 覆盖增加 | 深度改变的像素 | 同深度明暗改变 |
| --- | --- | --- | --- | --- | --- |
| fit | 0 / 0.5 / 1 / 2 | 212 / 11 / 12 / 13 | 0 / +26 / +71 / +247 | 0 / 288 / 422 / 1010 | 0 / 532 / 1275 / 2308 |
| fit:4 | 0 / 0.5 / 1 / 2 | 215 / 3 / 3 / 7 | 0 / +5 / +9 / +23 | 0 / 446 / 1010 / 2593 | 0 / 450 / 785 / 2361 |
| fit:20 | 0 / 0.5 / 1 / 2 | 192 / 3 / 2 / 2 | 0 / +171 / +171 / +171 | 0 / 810 / 2099 / 3422 | 0 / 643 / 707 / 721 |

- 裂缝的形态：局部裁切图（放大 4 倍）中是沿两个 face 交界的零星单像素针孔，没有连续的缝。0.5 px
  已经消除 95%～98%，再加大 δ 不再减少间隙，只增加重叠。
- 轮廓外扩：覆盖增加最多是覆盖像素的 0.07%（fit 下 +26 / 25 万），凸棱处的轮廓变化看不出来。
- 重叠区：外扩出的边缘在同一深度显示另一个 face 时，明暗会变化（尖棱两侧法线不同）。0.5 px 时这样的
  像素占覆盖像素的 0.02%～0.2%，只在裁切图里检查过裂缝位置，没有专门检查 G1 边上的 z-fighting 图像。
- 外扩后残留的几个间隙像素没有逐个查明是否为模型本身的窄缝。

### T5：收尾

- 性能：fit 和近景各测一次，重点看长尾 patch（段数最多的那些）的片元开销，决定要不要补一个备选
  加速结构。
- 更新 plan.md 和相关 skill 文档。

#### T5 结果

绘制时间（`steam_trim_draw_cost`，1920×1080，深度测试，M5 Pro，每次提交 10 次绘制取平均，再取多次
提交的中位数，误差约 ±0.1 ms）：

| 视角 | 整个 cell，不分类 | 分类、不裁剪 | band 加格子 | 全部段（无加速） | 最多 64 条 band |
| --- | --- | --- | --- | --- | --- |
| fit | 0.53 ms | 0.53 ms | 0.66 ms | 0.89 ms | 0.67～0.77 ms |
| fit:4 | 1.81 ms | 1.43 ms | 1.93 ms | 2.76 ms | 2.09 ms |
| fit:20 | 0.53 ms | 0.53 ms | 0.67 ms | 2.13 ms | 0.68 ms |
| fit:80 | 0.54 ms | 0.54 ms | 0.67 ms | 1.56 ms | 0.68 ms |

- T4 的第一版对环外片元遍历全部段求距离，fit:20 / fit:80 的裁剪绘制分别为 1.43 / 1.17 ms，其中不外扩
  时只要 0.68 / 0.70 ms。距离开销占了大头，于是改为按外扩范围收集候选段，得到上表的数字。
- CPU 复现可见裁剪片元（无深度，最后写入者）的点查询访问段数：

  | 视角 | 平均访问段数 | 落在常数格 | 段数 > 32 的 patch 的片元占比 / 平均访问 | 最多 64 条 band 时平均访问 |
  | --- | --- | --- | --- | --- |
  | fit | 0.95 | 85% | 18% / 3.9 | 0.46 |
  | fit:4 | 1.9 | 71% | 14% / 9.7 | 0.91 |
  | fit:20 | 2.7 | 74% | 20% / 12.2 | 1.03 |
  | fit:80 | 4.5 | 65% | 24% / 17.0 | 1.67 |

结论：

- 暂不需要备选加速结构（RAVG 格子）。裁剪带来的额外绘制时间不超过约 0.13 ms，只有 fit:4（170 万个
  可见裁剪片元）多 0.5 ms。长尾 patch 承担了大部分访问，但访问数不是瓶颈：band 增加到 64 条时访问减半，
  绘制时间不变，表大小却增加 17%（32.9 万 → 38.6 万字），所以保留 16 条。
- 加速结构的收益主要来自预分类格子（65%～85% 的片元不求交）和距离候选的范围限制。全部段的版本在
  近景下慢 2～3 倍。
- 在裁剪更复杂的模型上需要重新评估；那时先试长尾 patch 单独加 band，再考虑 RAVG 格子（有专利问题，
  见 trim-research.md）。

文档：plan.md 的"后续阶段"、本文件的设计小节、crate README 已更新。skill 文档（`.claude/skills`）里
没有涉及参数曲面细分或裁剪的内容，不需要更新。

## 未决问题

已解决：根判定码的符号约定（T0）；长尾是否需要备选结构（T5：暂不需要）；δ 的取值（T4：0.5 px）；
固定上限 r（T4：不需要，由片元给出范围）。

- 3D 容差：默认取模型容差，多数 patch 的 uv 容差被上限 1e-3 截住。裂缝裁切图里没有看到由此引起的
  裁剪边偏移，但没有专门测量裁剪边与相邻 face 的边在屏幕上的偏差。
- 外扩与 plan.md 后续"跨 face watertight"的关系：几何上做到 watertight 之后，外扩可能不再需要。
- 远景下部分覆盖的叶子过度光栅化：fit 视角 82% 的片元走裁剪 pipeline，其中约 38% 被 discard，原因是
  根矩形几乎总是部分覆盖。可以考虑加拆分判据：部分覆盖且环外面积大的叶子继续拆。
- 覆盖率抗锯齿（见"后续"一节）。
- 外扩后残留的个别间隙像素的成因，以及 G1 边上的 z-fighting 没有单独的视觉检查。
