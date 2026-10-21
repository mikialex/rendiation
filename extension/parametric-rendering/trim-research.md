# 参数曲面裁剪调研

为 [plan-trim.md](plan-trim.md) 做的调研：业界和学术界怎么在 GPU 上渲染带裁剪的参数曲面，各方案和
我们的管线（GPU 自适应细分 + 逐片元 `(patch_id, u, v)`）怎么结合，最后给出推荐。

数据背景（steam-controller，详见 plan-trim.md）：带裁剪 patch 每个只有 p50 4、p99 8 条曲线（最多 138），
曲线以直线和非有理三次为主；近似成二次后最长尾的 patch 有 168～234 段，8 条 band 时最满的 band
仍有 60 段，说明长尾 patch 的曲线集中在很窄的 v 范围内。

## 方案分类

| 类别 | 代表 | 边界精度 | 跨 face 裂缝 | 主要代价 |
| --- | --- | --- | --- | --- |
| 裁剪纹理 | Guthe 2005 | 受纹理分辨率限制，需视角相关地重建 | 有 | 内存、近景失真 |
| 逐片元直接裁剪 | Schollmeyer 2009 / 2019，Claux 2012，ETER / WATER，Slug 和 RAVG（矢量图形） | 逐像素精确 | 有，要另外处理 | 片元开销、discard |
| 细分贴合裁剪边 | CAD 内核的静态三角化（如 OCCT BRepMesh），Balázs 2004，research.md 的 A-B 方向 | 由细分误差决定 | 可以做到无缝 | GPU 上的非凸带孔三角化很难 |
| 3D 邻面裁剪 | SolidWorks 专利 US8982121 | 精确 | 无（用相邻 face 的曲面做判定） | 需要光线投射，专利到 2033 |

逐片元裁剪是目前实时渲染大规模 CAD 模型的主流路线（ETER / WATER 也是），我们的方向没有问题。
下面按"点分类""加速结构""抗锯齿""裂缝"几个子问题对比具体做法。

## 各方案细节

### Schollmeyer & Fröhlich 2009：Direct Trimming

- 裁剪曲线保持原始阶数（有理 Bézier），在 u、v 两个方向的极值处拆成双单调段。
- 加速结构（double binary partition）：按段的端点 v 把参数域切成水平条（slab），条内所有单调段都
  横贯整条，彼此在 u 方向全序。片元先二分找到所在的条，再在条内二分找到紧邻右侧的段，交点数等于
  右侧段的个数，最后在那一段上二分求交，判断点在它的哪一侧。
- 优点：O(log n)，任意阶数，不近似。缺点：两层二分加上求交时的迭代二分，取数之间互相依赖，分支多；
  条数和每条的段列表最坏是 O(n²) 的内存。

### Schollmeyer & Fröhlich 2019：Efficient and Anti-Aliased Trimming

依据开源实现 gpucast（作者本人的仓库）中的 `trimming_contour_kd.glsl`、`pre_classification.glsl`、
`classification_to_coverage.glsl`。论文 PDF 当时打不开，未读正文。

- 点分类分三层：
  - 先用外环包围盒判断；
  - 再查每个 patch 一张的预分类纹理（每格存全内、全外或不确定）；
  - 不确定的格子进 kd-tree，叶子存"不与叶子重叠的轮廓贡献的交点数"和重叠的单调轮廓列表。在
    轮廓上二分找到对应的曲线，再在曲线上二分求交。
- 轮廓（contour）：相邻的单调曲线段合并成一条单调链，二分查找的对象从"段"变成"链"，减少列表长度。
- 抗锯齿：用 `dFdx/dFdy(uv)` 组成雅可比 J，在判定点附近用曲线上的点和包围盒点估一条切线，变换到
  屏幕空间，得到像素单位的有符号距离和角度，再查一张二维预积分覆盖率纹理（prefilter）。离曲线
  超过约 1/4 个像素对角线时直接返回中心点的分类结果。
- 论文自述：相比早期方法更精确、内存更省，抗锯齿在质量和性能上优于 MSAA 和屏幕空间 AA。

### ETER（SIGGRAPH 2023）与 WATER（SIGGRAPH Asia 2025）

依据 WATER 公开仓库里的 `record_trim.frag` / `visibility_trim.frag`。两篇论文本身没有细读，裁剪部分
是从代码反推出来的。

- 每个 Bézier patch 固定一张 7×7 的格子（和 ETER 的 7×7 细分网格对齐），格子查偏移表：
  - 小于 2 的值直接表示全内或全外；
  - 其余进入一棵最多两层的 BSP，分割线是任意方向的直线（存法线角和截距），叶子仍可能是常数；
  - 剩下的叶子是一条单调曲线链，带若干"空白区间"（区间内直接给出覆盖值），否则二分找到段，在该段
    预采样的折线上定位，最后用叉积判断点在哪一侧。
- 曲线在预处理时被采样成折线，精度取决于采样率；判定结果只有符号，没有距离，所以没有抗锯齿。
- research.md 记录了 ETER 没有解决裁剪导致的缝隙，靠保守光栅化和 visibility buffer 按深度和 patch ID
  在像素级挑选结果。WATER 的 watertight 主要针对细分的 patch 边界，从代码看裁剪仍是片元级（这一点
  未读论文，需要核实）。
- 启示：SOTA 系统也采用"粗格子预分类 + 局部曲线"的结构，而且格子分辨率和细分网格对齐。

### Claux et al. 2012 / 2014

- 2012（VMV）：多分辨率的 quadtree 裁剪结构，用近似的矢量表示。远处片元只需浅层查询，近景交给
  硬件细分。
- 2014（CGF，Crack-free Rendering of Dynamically Tesselated B-Rep Models）：细分 + 光线投射 + GPU
  直接裁剪。用深度在图像空间检测裂缝，填补方式有两种：图像空间里做一次简单分类，或者对裂缝像素
  在物体空间选择性地做光线投射。论文称后者比全场景光线投射快 2～6 倍、质量相同。

### Slug（Lengyel，JCGT 2017）

依据参考 shader（`EricLengyel/Slug`，MIT）和作者的 "A Decade of Slug" 博文。

- 专利 US 10,373,352 已于 2026-03-17 放弃剩余期限，进入公有领域，参考 shader 以 MIT 协议发布。
- 曲线全部是二次，控制点减去采样点后，用三个 y 的符号查 `0x2E74` 表得到根判定码。shader 实现是
  `(0x2E74 >> shift) & 0x0101`，bit 0 和 bit 8 分别表示两个根是否计入。判定只依赖符号，首尾相接、
  切点都不需要 epsilon。博文说这部分从 2017 年到现在没有改过。
- 求根：判别式夹到 0；`|a| < 1/65536` 时走线性分支。
- band：水平 band 内的曲线按 max x 降序排序，max x 已在像素左边超过半个像素时提前退出；垂直 band
  对称处理。早期的 band split（每条 band 存两个方向的列表）已删掉：它对大字形有帮助，但增加了像素
  shader 的分支发散，删掉后 band 数据也减半。
- 覆盖率：每个根贡献 `saturate(r·ppe + 0.5)`，同时记一个权重 `saturate(1 - 2|r·ppe|)`（根离像素越近权重
  越大），水平和垂直结果按权重混合，再与 `min(|xcov|, |ycov|)` 取 max。`ppe = 1 / fwidth(coord)`，按
  两个轴分别计算，隐含的假设是参数到屏幕的映射没有明显的斜切。
- 新增的动态外扩：vertex shader 每帧把包围多边形向外扩半个像素，保证部分覆盖像素的中心落在多边形
  内。对我们来说，叶子网格本身就覆盖整个 cell，不需要这一步。

### Nehab & Hoppe 2008：Random-Access Vector Graphics（RAVG）

依据作者主页上 docx 版本的正文。

- 画面切成粗格子，每个格子存一条变长的"格子流"：裁剪到扩展格子内的线段和二次段。像素 shader
  只解释所在格子的流。
- 射线朝 +x，所以格子上方、左方、下方的段都可以省略。为了让 +x 射线在格子内得到正确的 winding：
  - 离开右边界的段，从交点补一条到格子右上角的辅助段；进入右边界的段，从右上角补到交点。
  - 段穿过格子下边界时在格子上记 Δh（向上 +1，向下 −1），然后每行从右往左做前缀和，在格子右边界
    补 |h| 条贯穿整格的竖直辅助段。
  - 结果是每个格子的判定只依赖自己的数据，开销与局部复杂度成正比，与整个形状无关。
- 三次曲线自适应细分成二次（与我们相同）。二次段中间点恰好使射线方程退化为线性时，编码阶段把它
  扰动 1 bit，从而省掉运行时分支（Slug 改用线性分支）。
- 距离：二次曲线转成隐式形式 `f = βδ − α²`（Bezout 结式）。先用 f 的一阶 Taylor 展开把采样点投影到
  曲线附近的 p'，再用参数反演求出 t，t 夹到 [0, 1] 内以正确处理端点，距离向量为 `p − b(t)`。它在空间
  上连续，在曲线附近足够准确；论文给出的 HLSL 约 32 条指令，只有两次除法，没有开方、循环和分支。
  三个控制点共线时分母为零，所以直线要走单独的分支。距离向量再用 J 换算到屏幕，做预滤波抗锯齿。
- 扩展格子：格子存所有影响扩展区域（多出 10～20%）的图元，这样抗锯齿时像素足迹不必跨格子。
- 专利：US7872648（Microsoft），Google Patents 显示有效期到 2029-11-17（推定，不是法律结论）。

### 其他

- Loop & Blinn 2005：对曲线三角形在片元里求隐式值。要先对裁剪区域做约束三角化，参数域是静态的，
  这一步可以在 CPU 上预先做好；难点在于把它和视角相关的细分叶子结合起来（与 research.md 的 A-B
  方向相同）。据 Slug 公有化新闻下的讨论，其专利在 2026 年 3 月到期（未核实）。
- NV_path_rendering（stencil-then-cover）：每帧要在参数域里光栅化曲线，和三维曲面的映射对不上，
  不适用。
- PaRas（SIGGRAPH 2025）：公开仓库里没有裁剪相关的代码。
- SolidWorks 专利 US8982121：参数域格子标记为空、满、边、顶点；边格和顶点格引用相邻 face 的曲面，
  在 3D 里判定，从而避免裁剪曲线近似带来的裂缝。思路有价值，但要光线投射，专利推定到 2033 年。

## 子问题对比

### 点分类

| 方法 | 曲线表示 | 单段求交 | 鲁棒性 |
| --- | --- | --- | --- |
| Schollmeyer | 原始阶数，双单调 | 迭代二分 | 依赖 epsilon 和迭代次数 |
| ETER / WATER | 预采样折线 | 叉积 | 精度取决于采样率 |
| RAVG | 二次（三次细分） | 闭式求根 | 编码时扰动控制点，运行时依赖普通的比较 |
| Slug | 二次 | 闭式求根 | 根判定码，不需要 epsilon |

结论：二次加 Slug 根判定码最适合 GPU，也最鲁棒。我们的数据是"每 patch 段数很少"，闭式求根加
无分支判定的优势最大。保留。

### 加速结构

| 方法 | 片元开销 | 预分类 | 预处理复杂度 |
| --- | --- | --- | --- |
| Slug 均匀 band | band 内、像素右侧的段（有提前退出） | 无 | 很低 |
| double binary partition | O(log n) 次依赖取数 | 无 | 中 |
| kd + 预分类（Schollmeyer 2019） | 预分类命中时 O(1)，否则 kd 遍历加轮廓二分 | 有 | 高 |
| 7×7 格子 + BSP + 单调链（ETER） | 格子 + 两层 BSP + 二分 | 有 | 高 |
| RAVG 格子 | 只扫格子内的段 | 自带（段列表为空的格子就是常数格） | 中：要把二次段裁剪到格子，并生成辅助段 |

结论：

- 对 p50 4 段的 patch，什么结构都差不多，关键是让大部分片元根本不走求交。所以预分类格子是收益
  最大的一项：每个 SOTA 系统都有它（gpucast 的预分类纹理、ETER 的 7×7 格子）。
- 长尾的问题在于局部密集，按 band 扫描解决不了，只有"预存远处贡献，只测局部"的结构能解决
  （kd 叶子的交点数、RAVG 的辅助段）。
- RAVG 格子同时提供预分类和局部求交，而且和我们的二进位 key 天然对齐：格子数取 2 的幂时
  `floor(uv·n)` 在 f32 里精确，片元一定严格落在所属格子的右边界左边。

### 抗锯齿与距离

| 方法 | 距离 | 对斜切映射 | 代价 |
| --- | --- | --- | --- |
| Slug | 水平、垂直两条射线的轴向距离，按权重混合 | `fwidth` 按轴分开，不处理斜切 | 两组 band |
| Schollmeyer 2019 | 判定点附近的切线，用 J⁻¹ 换算，查预积分纹理 | 正确 | 一次纹理查询 |
| RAVG | 隐式形式加反演的近似最近点距离，用 J 换算 | 正确 | 每段几次乘加 |

结论：曲面上的参数映射经常各向异性且带斜切，Slug 的 `fwidth` 做法不够。推荐 RAVG 的近似距离：
每段都能算，与射线方向无关，端点处理正确。也就是：一次遍历同时得到 winding（射线）和最小距离
向量（近似最近点），用 J⁻¹ 换算成像素距离，再用半平面盒滤波的闭式公式或预积分表得到覆盖率。
这样只需要一组水平 band 或格子，不需要垂直 band。注意：做抗锯齿时，格子要用扩展格子（RAVG 的
10～20% 外扩），否则格子边界附近会漏掉相邻格子里的段。

### 跨 face 裂缝

| 方法 | 说明 |
| --- | --- |
| 外扩 δ 像素 | 有了像素距离后最简单；不透明渲染下重叠无害，凸棱处轮廓最多外扩 δ |
| 图像空间检测与填补（Claux 2014，ETER） | 用深度和 patch ID 检测裂缝像素再填补，需要额外的 pass 或 visibility buffer |
| 邻面曲面判定（SolidWorks 专利） | 原理上无缝，需要光线投射，专利未过期 |
| 几何 watertight | 裁剪边作为细分的约束边，两侧共享一条 3D 边的折线（plan.md 的"边细分"方向） |

结论：先外扩，长期走几何 watertight。

## 推荐（已写入 plan-trim.md）

- **点分类**：二次段加 Slug 根判定码。
- **加速结构抽象成可替换的实现**：band、格子、树都满足同一个约定，即点 p 的
  `winding = w0 + Σ 候选段的交点数`，另加矩形查询和距离约定（见 plan-trim.md）。第一个实现是
  Slug 均匀 band 加 2 bit 预分类格子：预处理最简单、没有专利问题，对占绝大多数的简单 patch 已经够用。
  RAVG 式格子和 Schollmeyer 2009 式不均匀条作为备选，T5 测出长尾确实有问题时再加。
- **距离**：RAVG 的近似最近点距离加 J⁻¹ 换算，用于 T4 的外扩，不需要垂直 band。
- **覆盖率抗锯齿**：暂不做，设计记录在 plan-trim.md 的"后续：覆盖率抗锯齿"。
- **裂缝**：T4 先外扩，同时记录裂缝的实际表现。几何 watertight 留给 plan.md 的跨 face 阶段。
- **叶子级剔除**：上面这些方案都没有在自适应细分的层级上按裁剪剔除工作项（ETER 的格子只服务于
  片元）。这一项是我们管线特有的收益，保留为 T3。

## 专利的实际影响

以下是对公开信息的整理，不是法律意见。

| 专利 | 内容 | 状态（Google Patents 推定） | 对我们 |
| --- | --- | --- | --- |
| Slug US 10,373,352 | 根判定码、band | 2026-03 放弃，公有领域 | 无影响 |
| RAVG US7872648（Microsoft） | 独立权利要求是"矢量图形图像的编码"：均匀粗网格，每个格子的图层和图元编码为变长 token 串 / texel program | 有效，推定 2029-11-17 到期；只有美国，没有其他国家的同族 | 只影响备选的 RAVG 式格子 |
| SolidWorks US8982121 | 参数域格子标记为空 / 满 / 边 / 顶点，用相邻 face 的曲面加光线投射判定 | 有效，推定 2033-12 到期 | 我们不用这个方案 |
| Loop-Blinn（Microsoft） | 曲线三角形的隐式求值 | 据报道 2026-03 到期（未核实） | 不用 |

不做商业考虑时的实际影响：

- 专利有地域性。RAVG 专利只在美国有效，在美国以外实施、分发不受它约束。
- 美国没有通用的非商业豁免：制造和使用本身就可能构成侵权。实验使用例外在 Madey v. Duke（2002）之后
  已经非常窄。中国（专利法"专为科学研究和实验而使用有关专利"）、德国等有科研例外，但一般只覆盖
  "研究这项发明本身"，不覆盖把它当作工具放进软件里使用。
- 现实中的执行：起诉要成本，赔偿按实际损失或合理许可费计算，非商业个人项目几乎没有赔偿基数，
  针对个人开源项目起诉的情况很少。现实风险主要在下游：公司使用者（包括作者本人以后把它用在工作中）
  做开源合规审查时，会避开含专利实现的代码，一些开源基金会和发行版也会拒收。
- 是否落入权利要求要逐条比对，我无法判断。我们的用途是参数域里的裁剪区域：单层、无颜色、贴在
  曲面上，和"矢量图形图像"的措辞有差别，但不能据此认为不落入。RAVG 的近似距离公式是否在从属权利
  要求里，我没有核实。
- 应对方式：备选实现放在抽象层后面，做成可选模块（单独的 feature 或 crate），默认关闭；默认路径
  只用公有领域的 Slug 方法和常见的预分类格子。等到 2029 年专利到期，再考虑把它设为默认。

## 来源

- [Direct Trimming of NURBS Surfaces on the GPU（Schollmeyer & Fröhlich 2009）](https://www.uni-weimar.de/de/gpucast/direct-trimming-of-nurbs-surfaces-on-the-gpu)
- [Efficient and Anti-aliased Trimming for Rendering Large NURBS Models（Schollmeyer & Fröhlich, TVCG 2019）](https://uni-weimar.de/de/medien/professuren/medieninformatik/vr/research/real-time-rendering/gpucast/efficient-and-anti-aliased-trimming)
- [gpucast 源码](https://github.com/scholli/gpucast)
- [ETER（Xiong et al., TOG 2023）](https://dl.acm.org/doi/10.1145/3592419)
- [WATER 项目页](http://staff.ustc.edu.cn/~lgliu/Projects/2025_WATER/index.html)，[代码](https://github.com/Zeng187/WATER_public)
- [An Efficient Trim Structure for Rendering Large B-Rep Models（Claux et al., VMV 2012）](https://diglib.eg.org/handle/10.2312/PE.VMV.VMV12.031-038)
- [Crack-free Rendering of Dynamically Tesselated B-Rep Models（Claux et al., CGF 2014）](https://diglib.eg.org/items/3c07a8c4-0528-4b27-9948-0ff60ffdc0f9)
- [Slug 参考 shader](https://github.com/EricLengyel/Slug)，[A Decade of Slug](https://terathon.com/blog/decade-slug.html)
- [Random-Access Rendering of General Vector Graphics（Nehab & Hoppe 2008）](https://hhoppe.com/proj/ravg/)，[专利 US7872648](https://patents.google.com/patent/US7872648)
- [GPU-based Trimming and Tessellation of NURBS and T-Spline Surfaces（Guthe et al. 2005）](https://www.mathematik.uni-marburg.de/~guthe/Publications/guthe-2005-gpu-based.pdf)
- [Efficient Trimmed NURBS Tessellation（Balázs, Guthe, Klein 2004）](https://dspace5.zcu.cz/handle/11025/1747?locale=en)
- [Direct rendering of CAD models on the GPU（专利 US8982121）](https://patents.google.com/patent/US8982121)
- [PaRas 代码](https://github.com/renjiec/Paras)
- [research.md](https://github.com/mikialex/notebook-public/blob/master/project/3d-parametric-rendering/research.md)
