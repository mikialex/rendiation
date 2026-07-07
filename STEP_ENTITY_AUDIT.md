# STEP Entity 字段审计报告

审计范围：`extension/step-reader/src/entities.rs` 定义的 STEP 实体 vs `extension/parametric-rendering/src/step/` 中的实际使用。

## 高影响（几何形状/尺寸错误）

### EdgeCurve

| 字段 | 状态 | 后果 |
|---|---|---|
| `edge_start` | **忽略** | 所有曲线类型（Line, Circle, Ellipse, BSpline, Bezier, Polyline）的转换都不考虑 edge 的起止顶点范围。仅有非正式的 Circle 裁剪 workaround |
| `edge_end` | **忽略** | 同上 |
| `edge_geometry` | 使用 | 通过 `resolve_edge_geometry_fallback_from_id` 解析 |
| `same_sense` | 使用 | 用于 `should_reverse` 计算 |

### TrimmedCurve

| 字段 | 状态 | 后果 |
|---|---|---|
| `trim_1` | **忽略** | [curve3d_convert.rs:53](../extension/parametric-rendering/src/step/curve3d_convert.rs#L53) 注释明确写 "Trim parameters are ignored for now"。所有 TrimmedCurve 按完整基础曲线渲染 |
| `trim_2` | **忽略** | 同上 |
| `sense_agreement` | **忽略** | 裁剪段的方向信息被丢弃 |
| `master_representation` | **忽略** | `TrimmingPreference` (Cartesian/Parameter/Unspecified) 从未被参考 |
| `basis_curve` | 使用 | 递归转换 |

### CompositeCurveSegment

| 字段 | 状态 | 后果 |
|---|---|---|
| `same_sense` | **忽略** | 当 segment 方向与 composite curve 方向相反时，segment 几何被错误正向遍历 |
| `transition` | **忽略** | `TransitionCode` (Discontinuous/Continuous/ContSameGradient/ContSameGradientSameCurvature) 被丢弃，可能在断点处产生缝隙 |
| `parent_curve` | 使用 | 递归转换 |

### SurfaceCurve

| 字段 | 状态 | 后果 |
|---|---|---|
| `master_representation` | **忽略** | 当 pcurve 是 authoritative representation 时，代码仍以 3D 曲线为准 |
| `curve_3d` | 使用 | 提取 3D 曲线 |
| `associated_geometry` | 部分使用 | 在 `resolve_edge_geometry_fallback_from_id` 中被丢弃 (`associated_geometry: Vec::new()`)，只通过独立的 `pcurve_refs` 路径访问 |

### 完全不支持的曲线类型

| 类型 | 后果 |
|---|---|
| `Hyperbola` | 返回 `Err(UnsupportedCurve)` |
| `Parabola` | 返回 `Err(UnsupportedCurve)` |
| `OffsetCurve3d` | 返回 `Err(UnsupportedCurve)`，`basis_curve`, `distance`, `self_intersect`, `ref_direction` 全部丢失 |

### 完全不支持的表面类型

| 类型 | 后果 |
|---|---|
| `SurfaceOfRevolution` | 返回 `Err(UnsupportedSurface)`，`swept_curve`, `axis_position` 全部丢失 |
| `OffsetSurface` | 返回 `Err(UnsupportedSurface)`，`basis_surface`, `distance`, `self_intersect` 全部丢失 |

---

## 中影响（形状大致正确，但边界/edge 可能出错）

### FaceBound / FaceOuterBound

| 字段 | 状态 | 后果 |
|---|---|---|
| `orientation` | **忽略** | STEP 明确标注 outer(true)/hole(false)，代码通过 2D signed area 重新计算——对退化/自交 loop 可能出错 |
| `bound` | 使用 | EdgeLoop 被遍历 |

### PcurveOrSurface

| 字段 | 状态 | 后果 |
|---|---|---|
| Pcurve 变体 | 使用 | 提取并处理 |
| Surface 变体 | **忽略** | 在 `extract_pcurve_refs_from_edge_curve` 中被静默跳过 |

### SphericalSurface / ToroidalSurface

| 字段 | 状态 | 后果 |
|---|---|---|
| 全部几何字段 | 使用 | 但总是生成完整几何（8/16 patches），即使 face 只覆盖一小部分。与 Cylinder/Cone 不同，Sphere/Torus 不通过 `compute_axis_v_extent_from_beziers` 计算实际覆盖范围 |

---

## 低影响（分类/元数据被忽略，几何仍然正确）

### 表面/曲线分类标记

| 实体 | 忽略的字段 | 备注 |
|---|---|---|
| BSplineSurfaceWithKnots | `surface_form` | `PlanarSurf`/`CylindricalSurf` 等，可用于针对性优化 |
| BezierSurface | `surface_form` | 同上 |
| BSplineCurveWithKnots | `curve_form` | `CircularArc`/`EllipticArc` 等 |
| BezierCurve | `curve_form` | 同上 |

### 周期性/闭合标记

| 实体 | 忽略的字段 |
|---|---|
| BSplineSurfaceWithKnots | `u_closed`, `v_closed` |
| BezierSurface | `u_closed`, `v_closed` |
| BSplineCurveWithKnots | `closed_curve` |

### Knot 分类

| 实体 | 忽略的字段 | 备注 |
|---|---|---|
| BSplineSurfaceWithKnots | `knot_spec` | 与 knots 冗余，无几何影响 |
| BSplineCurveWithKnots | `knot_spec` | 同上 |

### Self-intersect / Label

| 实体 | 忽略的字段 |
|---|---|
| 所有实体 | `self_intersect` |
| 所有实体 | `label` (仅用于 debug 输出) |

---

## 根因总结

核心问题是 `convert_any_curve_to_beziers` 的函数签名：

```rust
pub fn convert_any_curve_to_beziers(
  curve: &CurveAny,  // 只有曲线几何，没有裁剪信息
) -> Result<Vec<RationalBezierCurve3d<f32>>, StepReadError>
```

**缺失的信息链：**
- EdgeCurve 的 `edge_start`/`edge_end` → 定义了 3D 顶点位置
- TrimmedCurve 的 `trim_1`/`trim_2` + `sense_agreement` → 定义了参数域裁剪范围
- CompositeCurveSegment 的 `same_sense` → 定义了段的方向

所有这些裁剪/范围信息在曲线转换管道中都被丢弃了，导致：
1. 圆总是转完整 360°（而非 edge 定义的弧）
2. 裁剪曲线总是转完整基础曲线
3. Composite 反向段总是正向遍历
