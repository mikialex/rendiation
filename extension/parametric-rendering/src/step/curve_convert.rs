use rendiation_algebra::*;
use rendiation_step_reader::entities::{
  self, Axis2Placement3d, BSplineCurveWithKnots, BezierCurve, CartesianPoint, Circle, CurveAny,
  Direction, Ellipse, Line, NonRationalBSplineCurve, Polyline, RationalBSplineCurve,
};
use rendiation_step_reader::ruststep::tables::IntoOwned;

use crate::curve3d::RationalBezierCurve3d;
use crate::step::StepReadError;

/// Convert a 90° circular arc to a single rational quadratic Bezier curve.
///
/// Weights: `(1, √2/2, 1)`, middle control point at the tangent intersection.
///
/// * `center` - Circle center in 3D
/// * `start_dir` - Unit vector from center to arc start (perpendicular to `normal`)
/// * `normal` - Unit normal vector of the circle plane
/// * `radius` - Circle radius
pub fn convert_90deg_circular_arc_to_bezier(
  center: Vec3<f32>,
  start_dir: Vec3<f32>,
  normal: Vec3<f32>,
  radius: f32,
) -> RationalBezierCurve3d<f32> {
  let end_dir = normal.cross(start_dir); // 90° CCW rotation
  let p0 = center + start_dir * radius;
  let p2 = center + end_dir * radius;

  // Middle control point: tangent intersection, distance = R / cos(45°) = R * √2 from center
  let w_mid: f32 = std::f32::consts::FRAC_1_SQRT_2; // cos(45°) = √2/2 ≈ 0.7071
  let mid_dir = (start_dir + end_dir).normalize();
  let p1 = center + mid_dir * radius / w_mid;

  let cp = vec![
    Vec4::new(p0.x, p0.y, p0.z, 1.0),
    Vec4::new(p1.x * w_mid, p1.y * w_mid, p1.z * w_mid, w_mid),
    Vec4::new(p2.x, p2.y, p2.z, 1.0),
  ];

  RationalBezierCurve3d::new(cp, 2)
}

/// Convert a full circle to 4 rational quadratic Bezier curves (4 × 90° arc).
pub fn convert_circle_to_bezier_curves(
  position: &Axis2Placement3d,
  radius: f64,
) -> Vec<RationalBezierCurve3d<f32>> {
  let center = cartesian_point_to_vec3(&position.location);
  let normal = direction_to_vec3(&position.axis);
  let ref_dir = axis2_x_dir(position);

  let radius_f32 = radius as f32;
  let mut curves = Vec::with_capacity(4);
  let mut start_dir = ref_dir.normalize();

  for _ in 0..4 {
    curves.push(convert_90deg_circular_arc_to_bezier(
      center, start_dir, normal, radius_f32,
    ));
    start_dir = normal.cross(start_dir);
  }

  curves
}

/// Convert an ellipse to 4 rational quadratic Bezier curves.
///
/// An ellipse can be seen as a scaled circle. We generate 4 90° arcs of
/// a unit circle and scale them by the two semi-axis lengths.
pub fn convert_ellipse_to_bezier_curves(
  position: &Axis2Placement3d,
  semi_axis1: f64,
  semi_axis2: f64,
) -> Vec<RationalBezierCurve3d<f32>> {
  let center = cartesian_point_to_vec3(&position.location);
  let normal = direction_to_vec3(&position.axis);
  let ref_dir = axis2_x_dir(position);

  let a = semi_axis1 as f32;
  let b = semi_axis2 as f32;
  let mut curves = Vec::with_capacity(4);
  let mut start_dir = ref_dir.normalize();

  for i in 0..4 {
    let end_dir = normal.cross(start_dir);

    // Ellipse radii in start/end directions
    let r_start = if i % 2 == 0 { a } else { b };
    let r_end = if i % 2 == 0 { b } else { a };

    let p0 = center + start_dir * r_start;
    let p2 = center + end_dir * r_end;

    let w_mid: f32 = std::f32::consts::FRAC_1_SQRT_2;

    // P1 is the intersection of the two tangent lines
    let tan0 = if i % 2 == 0 {
      end_dir * b // Y direction, scaled by other axis
    } else {
      end_dir * a
    };
    let tan2 = if i % 2 == 0 {
      start_dir * (-a)
    } else {
      start_dir * (-b)
    };

    // Solve P0 + s*tan0 = P2 + t*tan2
    let d = p2 - p0;
    let cross_tan = tan0.x * tan2.y - tan0.y * tan2.x;
    let s = if cross_tan.abs() > 1e-10 {
      (d.x * tan2.y - d.y * tan2.x) / cross_tan
    } else {
      0.0
    };
    let p1_exact = p0 + tan0 * s;

    let cp = vec![
      Vec4::new(p0.x, p0.y, p0.z, 1.0),
      Vec4::new(
        p1_exact.x * w_mid,
        p1_exact.y * w_mid,
        p1_exact.z * w_mid,
        w_mid,
      ),
      Vec4::new(p2.x, p2.y, p2.z, 1.0),
    ];

    curves.push(RationalBezierCurve3d::new(cp, 2));
    start_dir = end_dir;
  }

  curves
}

/// Convert a straight line to a single degree-1 Bezier curve.
pub fn convert_line_to_bezier(
  pnt: &CartesianPoint,
  dir: &entities::Vector,
) -> RationalBezierCurve3d<f32> {
  let p0 = cartesian_point_to_vec3(pnt);
  let d = direction_to_vec3(&dir.orientation);
  let mag = dir.magnitude as f32;
  let p1 = p0 + d * mag;

  let cp = vec![
    Vec4::new(p0.x, p0.y, p0.z, 1.0),
    Vec4::new(p1.x, p1.y, p1.z, 1.0),
  ];

  RationalBezierCurve3d::new(cp, 1)
}

/// Convert a B-spline curve (unweighted) to rational Bezier curves via knot insertion.
///
/// * `degree` - Curve degree
/// * `control_points` - 3D control points
/// * `knots` - Knot vector (length = n + degree + 1 where n = control_points.len())
pub fn convert_bspline_curve_to_bezier(
  degree: usize,
  control_points: Vec<Vec3<f32>>,
  knots: Vec<f32>,
) -> Vec<RationalBezierCurve3d<f32>> {
  let knots = fix_curve_knot_length(knots, control_points.len(), degree);
  let nurbs = crate::curve3d::NurbsCurve3d::from_unweighted(control_points, degree, knots);
  nurbs.to_bezier_curves()
}

/// Convert a rational B-spline curve to rational Bezier curves via knot insertion.
///
/// * `degree` - Curve degree
/// * `control_points` - 3D control points (unweighted coordinates)
/// * `weights` - Weight per control point
/// * `knots` - Knot vector
pub fn convert_rational_bspline_curve_to_bezier(
  degree: usize,
  control_points: Vec<Vec3<f32>>,
  weights: Vec<f32>,
  knots: Vec<f32>,
) -> Vec<RationalBezierCurve3d<f32>> {
  let count = control_points.len();
  let weighted_cp: Vec<Vec4<f32>> = control_points
    .into_iter()
    .zip(weights.iter())
    .map(|(p, &w)| Vec4::new(p.x * w, p.y * w, p.z * w, w))
    .collect();

  let knots = fix_curve_knot_length(knots, count, degree);
  let nurbs = crate::curve3d::NurbsCurve3d::new(weighted_cp, count, degree, knots);
  nurbs.to_bezier_curves()
}

/// Defensive knot-length fix. See `fix_knot_length` in surface_convert.rs
/// for the rationale (STEP exporter inconsistencies in knot multiplicities).
fn fix_curve_knot_length(mut knots: Vec<f32>, count: usize, degree: usize) -> Vec<f32> {
  let expected = count + degree + 1;
  if knots.len() < expected {
    // Too short: pad by repeating the last knot value (clamped extension)
    let last = *knots.last().unwrap_or(&0.0);
    knots.resize(expected, last);
  } else if knots.len() > expected {
    knots.truncate(expected);
  }
  knots
}

/// Convert a standalone Bezier curve (already in Bezier form, no knots) to our type.
pub fn convert_bezier_curve_direct(curve: &BezierCurve) -> Vec<RationalBezierCurve3d<f32>> {
  let degree = curve.degree as usize;
  let points: Vec<Vec3<f32>> = curve
    .control_points_list
    .iter()
    .map(cartesian_point_to_vec3)
    .collect();

  vec![RationalBezierCurve3d::from_unweighted(points, degree)]
}

/// Top-level dispatch: convert any STEP curve into our rational Bezier curves.
///
/// Recursively handles `TrimmedCurve`, `CompositeCurve`, and `SurfaceCurve` by
/// extracting the underlying geometry.
pub fn convert_any_curve_to_bezier(
  curve: &CurveAny,
) -> Result<Vec<RationalBezierCurve3d<f32>>, StepReadError> {
  match curve {
    CurveAny::Line(line) => Ok(vec![convert_line_to_bezier(&line.pnt, &line.dir)]),
    CurveAny::Circle(circle) => Ok(convert_circle_to_bezier_curves(
      &circle.position,
      circle.radius,
    )),
    CurveAny::Ellipse(ellipse) => Ok(convert_ellipse_to_bezier_curves(
      &ellipse.position,
      ellipse.semi_axis1,
      ellipse.semi_axis2,
    )),
    CurveAny::BSplineCurveWithKnots(b) => {
      let degree = b.degree as usize;
      let control_points: Vec<Vec3<f32>> = b
        .control_points_list
        .iter()
        .map(cartesian_point_to_vec3)
        .collect();
      let knots: Vec<f32> = b.knots.iter().map(|&k| k as f32).collect();
      Ok(convert_bspline_curve_to_bezier(
        degree,
        control_points,
        knots,
      ))
    }
    CurveAny::BezierCurve(b) => Ok(convert_bezier_curve_direct(b)),
    CurveAny::RationalBSplineCurve(r) => {
      let (degree, control_points, weights, knots) = extract_rational_bspline_curve_data(r)?;
      Ok(convert_rational_bspline_curve_to_bezier(
        degree,
        control_points,
        weights,
        knots,
      ))
    }
    CurveAny::TrimmedCurve(t) => {
      // Extract the basis curve and convert it. Trim parameters are ignored
      // for now — they can be used later to clip the resulting Bezier curves.
      convert_any_curve_to_bezier(&t.basis_curve)
    }
    CurveAny::CompositeCurve(c) => {
      let mut result = Vec::new();
      for segment in &c.segments {
        let mut seg_curves = convert_any_curve_to_bezier(&segment.parent_curve)?;
        result.append(&mut seg_curves);
      }
      Ok(result)
    }
    CurveAny::SurfaceCurve(sc) => {
      // SurfaceCurve wraps a 3D curve plus pcurves. For 3D curve conversion,
      // we only care about the 3D geometry.
      convert_any_curve_to_bezier(&sc.curve_3d)
    }
    CurveAny::Polyline(poly) => {
      let mut result = Vec::new();
      let pts: Vec<Vec3<f32>> = poly.points.iter().map(cartesian_point_to_vec3).collect();
      for pair in pts.windows(2) {
        let cp = vec![
          Vec4::new(pair[0].x, pair[0].y, pair[0].z, 1.0),
          Vec4::new(pair[1].x, pair[1].y, pair[1].z, 1.0),
        ];
        result.push(RationalBezierCurve3d::new(cp, 1));
      }
      Ok(result)
    }
    CurveAny::Pcurve(_) => {
      // Pcurves are 2D parametric curves; they should be handled by
      // the dedicated pcurve extraction path, not by 3D curve conversion.
      Err(StepReadError::ConversionError(
        "Pcurve is 2D; should be handled by pcurve extraction path".into(),
      ))
    }
    CurveAny::Hyperbola(_) => Err(StepReadError::UnsupportedCurve {
      entity_type: "Hyperbola",
      id: 0,
    }),
    CurveAny::Parabola(_) => Err(StepReadError::UnsupportedCurve {
      entity_type: "Parabola",
      id: 0,
    }),
    CurveAny::OffsetCurve3d(_) => Err(StepReadError::UnsupportedCurve {
      entity_type: "OffsetCurve3d",
      id: 0,
    }),
  }
}

// --- Internal helpers ---

/// Extract degree, unweighted control points, weights, and knots from a
/// `RationalBSplineCurve` entity. Handles all four non-rational subtypes.
fn extract_rational_bspline_curve_data(
  r: &RationalBSplineCurve,
) -> Result<(usize, Vec<Vec3<f32>>, Vec<f32>, Vec<f32>), StepReadError> {
  match &r.non_rational_b_spline_curve {
    NonRationalBSplineCurve::BSplineCurveWithKnots(b) => {
      extract_from_bspline_curve(b, &r.weights_data)
    }
    NonRationalBSplineCurve::BezierCurve(b) => {
      let degree = b.degree as usize;
      let control_points: Vec<Vec3<f32>> = b
        .control_points_list
        .iter()
        .map(cartesian_point_to_vec3)
        .collect();
      let n = control_points.len();
      let weights: Vec<f32> = if r.weights_data.is_empty() {
        vec![1.0; n]
      } else {
        r.weights_data.iter().map(|&w| w as f32).collect()
      };
      // Bezier curve: clamped knot vector
      let knots = build_clamped_knots(degree, n);
      Ok((degree, control_points, weights, knots))
    }
    NonRationalBSplineCurve::QuasiUniformCurve(b) | NonRationalBSplineCurve::UniformCurve(b) => {
      extract_from_bspline_curve(b, &r.weights_data)
    }
  }
}

fn extract_from_bspline_curve(
  b: &BSplineCurveWithKnots,
  weights_data: &[f64],
) -> Result<(usize, Vec<Vec3<f32>>, Vec<f32>, Vec<f32>), StepReadError> {
  let degree = b.degree as usize;
  let control_points: Vec<Vec3<f32>> = b
    .control_points_list
    .iter()
    .map(cartesian_point_to_vec3)
    .collect();
  let n = control_points.len();
  let weights: Vec<f32> = if weights_data.is_empty() {
    vec![1.0; n]
  } else {
    weights_data.iter().map(|&w| w as f32).collect()
  };

  // Expand knot multiplicities into full knot vector
  let knots = expand_knots(&b.knots, &b.knot_multiplicities);
  Ok((degree, control_points, weights, knots))
}

/// Expand compressed knots + multiplicities into a full knot vector.
fn expand_knots(knots: &[f64], multiplicities: &[i64]) -> Vec<f32> {
  let mut result = Vec::new();
  for (&k, &m) in knots.iter().zip(multiplicities.iter()) {
    for _ in 0..m {
      result.push(k as f32);
    }
  }
  result
}

/// Build a clamped knot vector for a Bezier curve (no interior knots).
fn build_clamped_knots(degree: usize, num_cp: usize) -> Vec<f32> {
  let n_knots = num_cp + degree + 1;
  let mut knots = Vec::with_capacity(n_knots);
  for _ in 0..=degree {
    knots.push(0.0);
  }
  for _ in degree + 1..n_knots - degree - 1 {
    knots.push(0.0); // No interior knots for Bezier
  }
  for _ in 0..=degree {
    knots.push(1.0);
  }
  // Fix: We need the correct number of knots
  let mut result = Vec::with_capacity(n_knots);
  for _ in 0..=degree {
    result.push(0.0);
  }
  for _ in 0..(n_knots - 2 * (degree + 1)) {
    result.push(0.5); // dummy
  }
  for _ in 0..=degree {
    result.push(1.0);
  }
  result.truncate(n_knots);
  result
}

// --- Point / Direction conversion helpers ---

pub(crate) fn cartesian_point_to_vec3(p: &CartesianPoint) -> Vec3<f32> {
  let coords = &p.coordinates;
  match coords.len() {
    3 => Vec3::new(coords[0] as f32, coords[1] as f32, coords[2] as f32),
    2 => Vec3::new(coords[0] as f32, coords[1] as f32, 0.0),
    1 => Vec3::new(coords[0] as f32, 0.0, 0.0),
    _ => Vec3::zero(),
  }
}

pub(crate) fn cartesian_point_to_vec2(p: &CartesianPoint) -> Vec2<f32> {
  let coords = &p.coordinates;
  match coords.len() {
    2 => Vec2::new(coords[0] as f32, coords[1] as f32),
    1 => Vec2::new(coords[0] as f32, 0.0),
    _ if coords.len() >= 3 => Vec2::new(coords[0] as f32, coords[1] as f32),
    _ => Vec2::zero(),
  }
}

pub(crate) fn direction_to_vec3(d: &Direction) -> Vec3<f32> {
  let ratios = &d.direction_ratios;
  match ratios.len() {
    3 => Vec3::new(ratios[0] as f32, ratios[1] as f32, ratios[2] as f32),
    2 => Vec3::new(ratios[0] as f32, ratios[1] as f32, 0.0),
    1 => Vec3::new(ratios[0] as f32, 0.0, 0.0),
    _ => Vec3::zero(),
  }
}

/// Get the X-direction from an Axis2Placement3D, or compute a default
/// perpendicular to the axis when ref_direction is `$` (None).
pub(crate) fn axis2_x_dir(position: &Axis2Placement3d) -> Vec3<f32> {
  if let Some(ref_dir) = &position.ref_direction {
    direction_to_vec3(ref_dir).normalize()
  } else {
    let axis = direction_to_vec3(&position.axis).normalize();
    // Pick a direction perpendicular to the axis
    if axis.x.abs() < 0.9 {
      axis.cross(Vec3::new(1.0, 0.0, 0.0)).normalize()
    } else {
      axis.cross(Vec3::new(0.0, 1.0, 0.0)).normalize()
    }
  }
}

// --- 2D curve extraction for pcurve path ---

use rendiation_step_reader::table::Table;

use crate::step::topology::FoundCurveHolder;

/// Extract 2D sample points from a curve entity referenced by `entity_id`.
///
/// Looks up the entity across all curve tables and samples the curve
/// uniformly. The resulting `Vec<Vec2<f32>>` is suitable for feeding to
/// `reconstruct_boundary` for quadratic Bezier fitting.
pub fn extract_2d_curve_points(
  table: &Table,
  entity_id: u64,
) -> Result<Vec<Vec2<f32>>, StepReadError> {
  let holder = crate::step::topology::find_2d_curve_holder(table, entity_id).ok_or(
    StepReadError::MissingEntity {
      entity_type: "2D curve",
      id: entity_id,
    },
  )?;

  match holder {
    FoundCurveHolder::Line(line) => {
      let l: Line = line
        .clone()
        .into_owned(table)
        .map_err(|e| StepReadError::ConversionError(format!("Line: {e}")))?;
      let p0 = cartesian_point_to_vec2(&l.pnt);
      let d = direction_to_vec3(&l.dir.orientation);
      let mag = l.dir.magnitude as f32;
      let p1 = Vec2::new(p0.x + d.x * mag, p0.y + d.y * mag);
      Ok(vec![p0, p1])
    }
    FoundCurveHolder::Polyline(poly) => {
      let p: Polyline = poly
        .clone()
        .into_owned(table)
        .map_err(|e| StepReadError::ConversionError(format!("Polyline: {e}")))?;
      Ok(p.points.iter().map(cartesian_point_to_vec2).collect())
    }
    FoundCurveHolder::BSplineCurveWithKnots(b) => {
      let curve: BSplineCurveWithKnots = b
        .clone()
        .into_owned(table)
        .map_err(|e| StepReadError::ConversionError(format!("BSplineCurve: {e}")))?;
      Ok(sample_2d_bspline_curve(&curve))
    }
    FoundCurveHolder::BezierCurve(b) => {
      let curve: BezierCurve = b
        .clone()
        .into_owned(table)
        .map_err(|e| StepReadError::ConversionError(format!("BezierCurve: {e}")))?;
      Ok(sample_2d_bezier_curve(&curve))
    }
    FoundCurveHolder::RationalBSplineCurve(r) => {
      let curve: RationalBSplineCurve = r
        .clone()
        .into_owned(table)
        .map_err(|e| StepReadError::ConversionError(format!("RationalBSpline: {e}")))?;
      Ok(sample_2d_rational_bspline_curve(&curve))
    }
    FoundCurveHolder::Circle(c) => {
      let circle: Circle = c
        .clone()
        .into_owned(table)
        .map_err(|e| StepReadError::ConversionError(format!("Circle: {e}")))?;
      Ok(sample_2d_circle(&circle))
    }
    FoundCurveHolder::Ellipse(e) => {
      let ellipse: Ellipse = e
        .clone()
        .into_owned(table)
        .map_err(|e| StepReadError::ConversionError(format!("Ellipse: {e}")))?;
      Ok(sample_2d_ellipse(&ellipse))
    }
  }
}

/// Sample a 2D B-spline curve uniformly.
fn sample_2d_bspline_curve(b: &BSplineCurveWithKnots) -> Vec<Vec2<f32>> {
  let points_2d: Vec<Vec2<f32>> = b
    .control_points_list
    .iter()
    .map(cartesian_point_to_vec2)
    .collect();
  let knots: Vec<f32> = expand_2d_knots(&b.knots, &b.knot_multiplicities);

  if points_2d.is_empty() || knots.is_empty() {
    return points_2d;
  }

  let degree = b.degree as usize;
  let samples_per_span = 32;
  let domain_start = knots[degree];
  let domain_end = knots[knots.len() - degree - 1];

  let mut result = Vec::new();
  let total_samples = (knots.len() - 2 * degree) * samples_per_span;
  for i in 0..=total_samples {
    let t =
      domain_start + (domain_end - domain_start) * (i as f32) / (total_samples as f32).max(1.0);
    result.push(evaluate_2d_bspline(&points_2d, degree, &knots, t));
  }
  result
}

fn evaluate_2d_bspline(points: &[Vec2<f32>], degree: usize, knots: &[f32], t: f32) -> Vec2<f32> {
  let n = points.len() - 1;
  let span = find_knot_span_2d(n, degree, t, knots);
  let basis = basis_functions_2d(span, t, degree, knots);

  let mut sum = Vec2::zero();
  for i in 0..=degree {
    let idx = span - degree + i;
    sum.x += basis[i] * points[idx].x;
    sum.y += basis[i] * points[idx].y;
  }
  sum
}

fn find_knot_span_2d(n: usize, p: usize, t: f32, knots: &[f32]) -> usize {
  if t >= knots[n + 1] {
    return n;
  }
  if t <= knots[p] {
    return p;
  }
  let mut low = p;
  let mut high = n + 1;
  let mut mid = (low + high) >> 1;
  while t < knots[mid] || t >= knots[mid + 1] {
    if t < knots[mid] {
      high = mid;
    } else {
      low = mid;
    }
    mid = (low + high) >> 1;
  }
  mid
}

fn basis_functions_2d(span: usize, t: f32, p: usize, knots: &[f32]) -> Vec<f32> {
  let mut n = vec![0.0f32; p + 1];
  n[0] = 1.0;
  let mut left = vec![0.0f32; p + 1];
  let mut right = vec![0.0f32; p + 1];
  for j in 1..=p {
    left[j] = t - knots[span + 1 - j];
    right[j] = knots[span + j] - t;
    let mut saved = 0.0;
    for r in 0..j {
      let denom = right[r + 1] + left[j - r];
      let temp = if denom.abs() > 1e-10 {
        n[r] / denom
      } else {
        0.0
      };
      n[r] = saved + right[r + 1] * temp;
      saved = left[j - r] * temp;
    }
    n[j] = saved;
  }
  n
}

fn sample_2d_bezier_curve(b: &BezierCurve) -> Vec<Vec2<f32>> {
  let points: Vec<Vec2<f32>> = b
    .control_points_list
    .iter()
    .map(cartesian_point_to_vec2)
    .collect();
  let samples = points.len() * 32;
  let mut result = Vec::new();
  for i in 0..=samples {
    let t = i as f32 / samples as f32;
    result.push(evaluate_2d_bezier(&points, t));
  }
  result
}

fn evaluate_2d_bezier(points: &[Vec2<f32>], t: f32) -> Vec2<f32> {
  let n = points.len();
  let mut q = points.to_vec();
  let one_minus_t = 1.0 - t;
  for r in 1..n {
    for i in 0..n - r {
      q[i] = q[i] * one_minus_t + q[i + 1] * t;
    }
  }
  q[0]
}

fn sample_2d_rational_bspline_curve(r: &RationalBSplineCurve) -> Vec<Vec2<f32>> {
  match &r.non_rational_b_spline_curve {
    NonRationalBSplineCurve::BSplineCurveWithKnots(b)
    | NonRationalBSplineCurve::QuasiUniformCurve(b)
    | NonRationalBSplineCurve::UniformCurve(b) => {
      let points_2d: Vec<Vec2<f32>> = b
        .control_points_list
        .iter()
        .map(cartesian_point_to_vec2)
        .collect();
      let knots: Vec<f32> = expand_2d_knots(&b.knots, &b.knot_multiplicities);
      let degree = b.degree as usize;
      let weights: Vec<f32> = if r.weights_data.is_empty() {
        vec![1.0; points_2d.len()]
      } else {
        r.weights_data.iter().map(|&w| w as f32).collect()
      };

      if points_2d.is_empty() {
        return points_2d;
      }

      let samples_per_span = 32;
      let domain_start = knots[degree];
      let domain_end = knots[knots.len() - degree - 1];
      let total_samples = (knots.len() - 2 * degree) * samples_per_span;

      let mut result = Vec::new();
      for i in 0..=total_samples {
        let t =
          domain_start + (domain_end - domain_start) * (i as f32) / (total_samples as f32).max(1.0);
        result.push(evaluate_2d_rational_bspline(
          &points_2d, &weights, degree, &knots, t,
        ));
      }
      result
    }
    NonRationalBSplineCurve::BezierCurve(b) => {
      let points: Vec<Vec2<f32>> = b
        .control_points_list
        .iter()
        .map(cartesian_point_to_vec2)
        .collect();
      let weights: Vec<f32> = if r.weights_data.is_empty() {
        vec![1.0; points.len()]
      } else {
        r.weights_data.iter().map(|&w| w as f32).collect()
      };
      let samples = points.len() * 32;
      let mut result = Vec::new();
      for i in 0..=samples {
        let t = i as f32 / samples as f32;
        result.push(evaluate_2d_rational_bezier(&points, &weights, t));
      }
      result
    }
  }
}

fn evaluate_2d_rational_bspline(
  points: &[Vec2<f32>],
  weights: &[f32],
  degree: usize,
  knots: &[f32],
  t: f32,
) -> Vec2<f32> {
  let n = points.len() - 1;
  let span = find_knot_span_2d(n, degree, t, knots);
  let basis = basis_functions_2d(span, t, degree, knots);

  let mut numer = Vec2::<f32>::zero();
  let mut denom = 0.0f32;
  for i in 0..=degree {
    let idx = span - degree + i;
    let b = basis[i] * weights[idx];
    numer.x += b * points[idx].x;
    numer.y += b * points[idx].y;
    denom += b;
  }
  if denom.abs() > 1e-10 {
    Vec2::new(numer.x / denom, numer.y / denom)
  } else {
    Vec2::zero()
  }
}

fn evaluate_2d_rational_bezier(points: &[Vec2<f32>], weights: &[f32], t: f32) -> Vec2<f32> {
  let n = points.len();
  let mut qw: Vec<(Vec2<f32>, f32)> = points
    .iter()
    .zip(weights.iter())
    .map(|(&p, &w)| (p, w))
    .collect();
  let one_minus_t = 1.0 - t;
  for r in 1..n {
    for i in 0..n - r {
      let (p0, w0) = qw[i];
      let (p1, w1) = qw[i + 1];
      let w = w0 * one_minus_t + w1 * t;
      let p = (p0 * (w0 * one_minus_t) + p1 * (w1 * t)) / w.max(1e-10);
      qw[i] = (p, w);
    }
  }
  qw[0].0
}

fn sample_2d_circle(c: &Circle) -> Vec<Vec2<f32>> {
  let center = cartesian_point_to_vec2(&c.position.location);
  let r = c.radius as f32;
  let samples = 128;
  let mut result = Vec::new();
  for i in 0..=samples {
    let angle = (i as f32) * 2.0 * std::f32::consts::PI / (samples as f32);
    result.push(Vec2::new(
      center.x + r * angle.cos(),
      center.y + r * angle.sin(),
    ));
  }
  result
}

fn sample_2d_ellipse(e: &Ellipse) -> Vec<Vec2<f32>> {
  let center = cartesian_point_to_vec2(&e.position.location);
  let a = e.semi_axis1 as f32;
  let b = e.semi_axis2 as f32;
  let samples = 128;
  let mut result = Vec::new();
  for i in 0..=samples {
    let angle = (i as f32) * 2.0 * std::f32::consts::PI / (samples as f32);
    result.push(Vec2::new(
      center.x + a * angle.cos(),
      center.y + b * angle.sin(),
    ));
  }
  result
}

fn expand_2d_knots(knots: &[f64], multiplicities: &[i64]) -> Vec<f32> {
  let mut result = Vec::new();
  for (&k, &m) in knots.iter().zip(multiplicities.iter()) {
    for _ in 0..m {
      result.push(k as f32);
    }
  }
  result
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn circle_90deg_arc_endpoints() {
    let center = Vec3::zero();
    let start_dir = Vec3::new(1.0, 0.0, 0.0);
    let normal = Vec3::new(0.0, 0.0, 1.0);
    let radius = 2.0;

    let curve = convert_90deg_circular_arc_to_bezier(center, start_dir, normal, radius);

    let p0 = curve.evaluate(0.0);
    let p1 = curve.evaluate(1.0);
    assert!((p0 - Vec3::new(2.0, 0.0, 0.0)).length() < 1e-5);
    assert!((p1 - Vec3::new(0.0, 2.0, 0.0)).length() < 1e-5);

    // Midpoint should be on the circle
    let mid = curve.evaluate(0.5);
    let dist_from_center = (mid - center).length();
    assert!(
      (dist_from_center - radius).abs() < 1e-4,
      "midpoint radius: {dist_from_center}"
    );
  }

  #[test]
  fn circle_90deg_arc_constant_radius() {
    let center = Vec3::new(1.0, 2.0, 3.0);
    let start_dir = Vec3::new(0.0, 1.0, 0.0);
    let normal = Vec3::new(1.0, 0.0, 0.0);
    let radius = 5.0;

    let curve = convert_90deg_circular_arc_to_bezier(center, start_dir, normal, radius);

    for t in [0.0, 0.1, 0.25, 0.5, 0.75, 0.9, 1.0] {
      let pt = curve.evaluate(t);
      let dist = (pt - center).length();
      assert!(
        (dist - radius).abs() < 1e-4,
        "t={t}: dist={dist}, expected {radius}"
      );
    }
  }

  #[test]
  fn line_conversion_endpoints() {
    let pnt = CartesianPoint {
      label: String::new(),
      coordinates: vec![0.0, 0.0, 0.0],
    };
    let dir_orientation = Direction {
      label: String::new(),
      direction_ratios: vec![1.0, 0.0, 0.0],
    };
    let dir = entities::Vector {
      label: String::new(),
      orientation: dir_orientation,
      magnitude: 3.0,
    };

    let curve = convert_line_to_bezier(&pnt, &dir);
    let p0 = curve.evaluate(0.0);
    let p1 = curve.evaluate(1.0);
    assert!((p0 - Vec3::new(0.0, 0.0, 0.0)).length() < 1e-5);
    assert!((p1 - Vec3::new(3.0, 0.0, 0.0)).length() < 1e-5);
  }
}
