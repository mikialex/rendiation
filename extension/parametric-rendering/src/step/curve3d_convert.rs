use rendiation_step_reader::entities::{
  self, Axis2Placement3d, BSplineCurveWithKnots, BezierCurve, CartesianPoint, CurveAny,
  NonRationalBSplineCurve, RationalBSplineCurve,
};

use super::*;
use crate::step::StepReadError;
use crate::*;

/// Top-level dispatch: convert any STEP curve into our rational Bezier curves.
///
/// Recursively handles `TrimmedCurve`, `CompositeCurve`, and `SurfaceCurve` by
/// extracting the underlying geometry.
pub fn convert_any_curve_to_beziers(
  curve: &CurveAny,
  start_vertex: Option<Vec3<f32>>,
  end_vertex: Option<Vec3<f32>>,
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
      // convert_any_curve_to_beziers(&t.basis_curve)
      todo!()
    }
    CurveAny::CompositeCurve(c) => {
      // let mut result = Vec::new();
      // for segment in &c.segments {
      //   let mut seg_curves = convert_any_curve_to_beziers(&segment.parent_curve)?;
      //   result.append(&mut seg_curves);
      // }
      // Ok(result)
      todo!()
    }
    CurveAny::SurfaceCurve(sc) => {
      // SurfaceCurve wraps a 3D curve plus pcurves. For 3D curve conversion,
      // we only care about the 3D geometry.
      // convert_any_curve_to_beziers(&sc.curve_3d)
      todo!()
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

/// Convert a 90° circular arc to a single rational quadratic Bezier curve.
///
/// Weights: `(1, √2/2, 1)`, middle control point at the tangent intersection.
///
/// * `center` - Circle center in 3D
/// * `start_dir` - Unit vector from center to arc start (perpendicular to `normal`)
/// * `normal` - Unit normal vector of the circle plane
/// * `radius` - Circle radius
fn convert_90deg_circular_arc_to_bezier(
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
fn convert_circle_to_bezier_curves(
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

  // Nudge the last arc's endpoint inward by a microscopic amount so the
  // circle does not close perfectly.  A closed circle produces a wrapped
  // tessellation polyline whose closing segment (360°→0°) creates a large
  // Δu jump that breaks downstream UV clipping.
  if let Some(last) = curves.last_mut() {
    let cp = last.control_points_mut();
    let n = cp.len();
    let eps: f32 = 1e-6 * radius_f32;
    cp[n - 1].x += eps;
  }

  curves
}

/// Convert an ellipse to 4 rational quadratic Bezier curves.
///
/// An ellipse can be seen as a scaled circle. We generate 4 90° arcs of
/// a unit circle and scale them by the two semi-axis lengths.
fn convert_ellipse_to_bezier_curves(
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

  // Same wrap-avoidance nudge as the circle converter (see comment there).
  if let Some(last) = curves.last_mut() {
    let cp = last.control_points_mut();
    let n = cp.len();
    let eps: f32 = 1e-6 * a.max(b);
    cp[n - 1].x += eps;
  }

  curves
}

/// Convert a straight line to a single degree-1 Bezier curve.
fn convert_line_to_bezier(
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
fn convert_bspline_curve_to_bezier(
  degree: usize,
  control_points: Vec<Vec3<f32>>,
  knots: Vec<f32>,
) -> Vec<RationalBezierCurve3d<f32>> {
  let knots = fix_curve_knot_length(knots, control_points.len(), degree);
  NurbsCurve3d::from_unweighted(control_points, degree, knots).to_bezier_curves()
}

/// Convert a rational B-spline curve to rational Bezier curves via knot insertion.
///
/// * `degree` - Curve degree
/// * `control_points` - 3D control points (unweighted coordinates)
/// * `weights` - Weight per control point
/// * `knots` - Knot vector
fn convert_rational_bspline_curve_to_bezier(
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
  NurbsCurve3d::new(weighted_cp, count, degree, knots).to_bezier_curves()
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
fn convert_bezier_curve_direct(curve: &BezierCurve) -> Vec<RationalBezierCurve3d<f32>> {
  let degree = curve.degree as usize;
  let points: Vec<Vec3<f32>> = curve
    .control_points_list
    .iter()
    .map(cartesian_point_to_vec3)
    .collect();

  vec![RationalBezierCurve3d::from_unweighted(points, degree)]
}

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

#[cfg(test)]
mod tests {
  use rendiation_step_reader::entities::Direction;

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
