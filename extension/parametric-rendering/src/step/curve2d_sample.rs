//! --- 2D curve extraction for pcurve path ---

use rendiation_step_reader::entities::{
  BSplineCurveWithKnots, BezierCurve, Circle, Ellipse, Line, NonRationalBSplineCurve, Polyline,
  RationalBSplineCurve,
};
use rendiation_step_reader::ruststep::tables::IntoOwned;
use rendiation_step_reader::table::Table;

use super::*;
use crate::*;

/// Extract 2D sample points from a curve entity referenced by `entity_id`.
///
/// Looks up the entity across all curve tables and samples the curve
/// uniformly.
pub fn extract_2d_curve_points(
  table: &Table,
  entity_id: u64,
) -> Result<ContinuousTrimPolyline, StepReadError> {
  let holder = find_2d_curve_holder(table, entity_id).ok_or(StepReadError::MissingEntity {
    entity_type: "2D curve",
    id: entity_id,
  })?;

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
      Ok(ContinuousTrimPolyline::new_by_single_segment(p0, p1))
    }
    FoundCurveHolder::Polyline(poly) => {
      let p: Polyline = poly
        .clone()
        .into_owned(table)
        .map_err(|e| StepReadError::ConversionError(format!("Polyline: {e}")))?;
      Ok(ContinuousTrimPolyline::new_from_hard_polylines(
        p.points.iter().map(cartesian_point_to_vec2).collect(),
      ))
    }
    FoundCurveHolder::BSplineCurveWithKnots(b) => {
      let curve: BSplineCurveWithKnots = b
        .clone()
        .into_owned(table)
        .map_err(|e| StepReadError::ConversionError(format!("BSplineCurve: {e}")))?;
      Ok(ContinuousTrimPolyline::single(sample_2d_bspline_curve(
        &curve,
      )))
    }
    FoundCurveHolder::BezierCurve(b) => {
      let curve: BezierCurve = b
        .clone()
        .into_owned(table)
        .map_err(|e| StepReadError::ConversionError(format!("BezierCurve: {e}")))?;
      Ok(ContinuousTrimPolyline::single(sample_2d_bezier_curve(
        &curve,
      )))
    }
    FoundCurveHolder::RationalBSplineCurve(r) => {
      let curve: RationalBSplineCurve = r
        .clone()
        .into_owned(table)
        .map_err(|e| StepReadError::ConversionError(format!("RationalBSpline: {e}")))?;
      Ok(ContinuousTrimPolyline::single(
        sample_2d_rational_bspline_curve(&curve),
      ))
    }
    FoundCurveHolder::Circle(c) => {
      let circle: Circle = c
        .clone()
        .into_owned(table)
        .map_err(|e| StepReadError::ConversionError(format!("Circle: {e}")))?;
      Ok(ContinuousTrimPolyline::single(sample_2d_circle(&circle)))
    }
    FoundCurveHolder::Ellipse(e) => {
      let ellipse: Ellipse = e
        .clone()
        .into_owned(table)
        .map_err(|e| StepReadError::ConversionError(format!("Ellipse: {e}")))?;
      Ok(ContinuousTrimPolyline::single(sample_2d_ellipse(&ellipse)))
    }
  }
}

/// Sample a 2D B-spline curve uniformly.
fn sample_2d_bspline_curve(b: &BSplineCurveWithKnots) -> NoEdgeContinuousTrimPolyline {
  let points_2d: Vec<Vec2<f32>> = b
    .control_points_list
    .iter()
    .map(cartesian_point_to_vec2)
    .collect();
  let knots: Vec<f32> = expand_2d_knots(&b.knots, &b.knot_multiplicities);

  if points_2d.is_empty() || knots.is_empty() {
    // todo, is this valid??
    return NoEdgeContinuousTrimPolyline::new_assume_no_edge(points_2d);
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
  NoEdgeContinuousTrimPolyline::new_assume_no_edge(result)
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

fn sample_2d_bezier_curve(b: &BezierCurve) -> NoEdgeContinuousTrimPolyline {
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
  NoEdgeContinuousTrimPolyline::new_assume_no_edge(points)
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

fn sample_2d_rational_bspline_curve(r: &RationalBSplineCurve) -> NoEdgeContinuousTrimPolyline {
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
        // todo, report error
        return NoEdgeContinuousTrimPolyline::new_assume_no_edge(points_2d);
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
      NoEdgeContinuousTrimPolyline::new_assume_no_edge(result)
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
      NoEdgeContinuousTrimPolyline::new_assume_no_edge(result)
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

fn sample_2d_circle(c: &Circle) -> NoEdgeContinuousTrimPolyline {
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
  NoEdgeContinuousTrimPolyline::new_assume_no_edge(result)
}

fn sample_2d_ellipse(e: &Ellipse) -> NoEdgeContinuousTrimPolyline {
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
  NoEdgeContinuousTrimPolyline::new_assume_no_edge(result)
}
