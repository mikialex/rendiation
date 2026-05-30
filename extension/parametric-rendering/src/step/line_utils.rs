use crate::*;

// ContinuousTrimPolyline, but verify loop closed
pub struct CompletedTrimPolyline(ContinuousTrimPolyline);

impl CompletedTrimPolyline {
  // return None if the line only has <= 2 points
  pub fn check_closed_from(line: ContinuousTrimPolyline) -> Option<Self> {
    let first = line.polylines.first()?.points.first().copied()?;
    let last = line.polylines.last()?.points.last().copied()?;

    let total_points: usize = line.polylines.iter().map(|p| p.points.len()).sum();
    if total_points <= 2 {
      return None;
    }

    if first.distance_to(last) > 1e-6 {
      return None;
    }

    Some(Self(line))
  }

  pub fn is_point_inside(&self, point: Vec2<f32>) -> bool {
    let polygon: Vec<Vec2<f32>> = self.iter_points().collect();
    point_in_polygon(point, &polygon)
  }

  pub fn iter_points(&self) -> impl Iterator<Item = Vec2<f32>> {
    let mut points = Vec::new();

    for (i, poly) in self.0.polylines.iter().enumerate() {
      if i == 0 {
        points.extend_from_slice(&poly.points);
      } else {
        points.extend_from_slice(&poly.points[1..]);
      }
    }

    if let Some(&first) = points.first() {
      points.push(first);
    }

    points.into_iter()
  }

  pub fn reconstruct_quadratic_bezier_curves(
    self,
    error_distance_tolerance: f32,
  ) -> Vec<QuadraticBezierCurve2d<f32>> {
    self
      .0
      .reconstruct_quadratic_bezier_curves(error_distance_tolerance)
  }
}

/// A single continuous(maybe contains edges) point polyline in UV space.
/// (will be reconstructed by one of more bezier curves)
///
/// the topology is line-strip
#[derive(Default)]
pub struct ContinuousTrimPolyline {
  // between each polyline there is no gap
  polylines: Vec<NoEdgeContinuousTrimPolyline>,
}

impl ContinuousTrimPolyline {
  pub fn single(single: NoEdgeContinuousTrimPolyline) -> Self {
    Self {
      polylines: vec![single],
    }
  }

  pub fn new_from_hard_polylines(points: Vec<Vec2<f32>>) -> Self {
    if points.len() < 2 {
      return Self::default();
    }
    Self {
      polylines: vec![NoEdgeContinuousTrimPolyline { points }],
    }
  }

  pub fn map(mut self, mapper: impl Fn(Vec2<f32>) -> Vec2<f32>) -> Self {
    for polyline in &mut self.polylines {
      for point in &mut polyline.points {
        *point = mapper(*point);
      }
    }
    self
  }

  pub fn new_by_single_segment(start: Vec2<f32>, end: Vec2<f32>) -> Self {
    Self {
      polylines: vec![NoEdgeContinuousTrimPolyline::line_segment(start, end)],
    }
  }
  pub fn connect_c_polyline(&mut self, other: ContinuousTrimPolyline) {
    for polyline in other.polylines {
      self.push_no_edge_polyline(polyline);
    }
  }

  pub fn push_no_edge_polyline(&mut self, mut polyline: NoEdgeContinuousTrimPolyline) {
    assert!(!polyline.is_degenerate());
    if let Some(last) = self.polylines.last() {
      let old_last = *last.points.last().unwrap();
      let new_start = *polyline.points.first().unwrap();
      let du = new_start.x - old_last.x;
      let dv = new_start.y - old_last.y;

      // Same periodic-boundary logic as fix_periodic_boundary_uv_jump,
      // applied between adjacent sub-polylines. When the previous polyline
      // ends at u≈1 and this one starts at u≈0 (or vice versa), shift the
      // entire incoming polyline to keep coordinates continuous rather
      // than inserting an invalid straight-line bridge across UV space.
      if du.abs() > 0.99 {
        for p in &mut polyline.points {
          p.x -= du.round();
        }
      } else if dv.abs() > 0.99 {
        for p in &mut polyline.points {
          p.y -= dv.round();
        }
      } else {
        assert!(
          old_last.distance_to(new_start) <= 1e-6,
          "polyline discontinuity: old_last={old_last:?}, new_start={new_start:?}, dist={}",
          old_last.distance_to(new_start)
        );
      }
    }

    self.polylines.push(polyline);
  }

  pub fn push_point(&mut self, point: Vec2<f32>) {
    if let Some(last) = self.polylines.last_mut() {
      last.push(point);
    } else {
      self.polylines.push(NoEdgeContinuousTrimPolyline {
        points: vec![point],
      });
    }
  }

  pub fn last_point(&self) -> Option<Vec2<f32>> {
    self.polylines.last()?.points.last().copied()
  }

  pub fn reverse(mut self) -> Self {
    self.polylines.reverse();
    for poly in &mut self.polylines {
      poly.points.reverse();
    }
    self
  }

  pub fn reconstruct_quadratic_bezier_curves(
    self,
    error_distance_tolerance: f32,
  ) -> Vec<QuadraticBezierCurve2d<f32>> {
    self
      .polylines
      .into_iter()
      .flat_map(|p| p.reconstruct_quadratic_bezier_curves(error_distance_tolerance))
      .collect()
  }
}

/// A single continuous and no edge point polyline in UV space.
/// (will be reconstructed by one of more bezier curves)
///
/// the topology is line-strip
#[derive(Default)]
pub struct NoEdgeContinuousTrimPolyline {
  points: Vec<Vec2<f32>>,
}

impl NoEdgeContinuousTrimPolyline {
  pub fn from_curve_sample(points: Vec<Vec2<f32>>) -> Self {
    Self { points }
  }

  pub fn line_segment(start: Vec2<f32>, end: Vec2<f32>) -> Self {
    assert!(start != end);
    Self {
      points: vec![start, end],
    }
  }

  pub fn is_degenerate(&self) -> bool {
    self.points.len() < 2
  }

  /// Fix periodic boundary wrapping for cylinder-like surfaces where u
  /// (or v) wraps from ~1 back to ~0 within a single continuous polyline.
  /// Skips line segments (2 points) since their endpoints may genuinely
  /// lie on opposite sides of the periodic boundary.
  pub fn fix_periodic_boundary_uv_jump(&mut self) {
    if self.points.len() <= 2 {
      return;
    }
    let mut u_off = 0.0;
    let mut v_off = 0.0;
    for i in 1..self.points.len() {
      let prev = Vec2::new(self.points[i - 1].x + u_off, self.points[i - 1].y + v_off);
      let du = self.points[i].x - prev.x;
      if du.abs() > 0.99 {
        u_off += if du > 0.0 { -1.0 } else { 1.0 };
      }
      let dv = self.points[i].y - prev.y;
      if dv.abs() > 0.99 {
        v_off += if dv > 0.0 { -1.0 } else { 1.0 };
      }
      self.points[i].x += u_off;
      self.points[i].y += v_off;
    }
  }

  pub fn push(&mut self, point: Vec2<f32>) {
    if let Some(last) = self.points.last() {
      if last == &point {
        return;
      }
    }
    self.points.push(point);
  }

  pub fn reconstruct_quadratic_bezier_curves(
    self,
    error_distance_tolerance: f32,
  ) -> Vec<QuadraticBezierCurve2d<f32>> {
    reconstruct_boundary(&self.points, error_distance_tolerance)
  }
}

/// Reconstructs a 2D boundary polyline as a minimal sequence of quadratic
/// Bézier curves.
///
/// Expects the input polyline to be tangent-continuous — sharp corners should
/// be handled by splitting the polyline at corner vertices before calling this
/// function.
///
/// Uses a greedy bottom-up strategy: starting from the first point, each
/// curve absorbs as many subsequent points as possible while keeping the
/// maximum distance from every point to the curve within
/// `error_distance_tolerance`.
pub fn reconstruct_boundary<T: Scalar>(
  points: &[Vec2<T>],
  error_distance_tolerance: T,
) -> Vec<QuadraticBezierCurve2d<T>> {
  if points.len() < 2 {
    return Vec::new();
  }

  let tolerance_sq = error_distance_tolerance * error_distance_tolerance;
  let mut curves = Vec::new();
  let mut start = 0;

  while start < points.len() - 1 {
    let mut end = start + 1;
    let mut best_p1 = Vec2::new(T::zero(), T::zero());

    while end < points.len() {
      let (p1, max_err_sq) = fit_quadratic_bezier_to_points(&points[start..=end]);
      if max_err_sq > tolerance_sq {
        break;
      }
      best_p1 = p1;
      end += 1;
    }

    curves.push(QuadraticBezierCurve2d {
      start: points[start],
      ctrl: best_p1,
      end: points[end - 1],
    });

    start = end - 1;
  }

  curves
}

/// Fit a single quadratic Bézier curve to a window of polyline points using
/// least squares.
///
/// The endpoints are pinned to the first and last point of the window.
/// Intermediate points are assigned chord-length parameters, and the
/// optimal control point is solved by minimizing Σ|Qᵢ - B(tᵢ)|².
///
/// Returns the optimal control point and the maximum squared error across
/// all points in the window.
fn fit_quadratic_bezier_to_points<T: Scalar>(points: &[Vec2<T>]) -> (Vec2<T>, T) {
  let n = points.len();
  debug_assert!(n >= 2);

  let p0 = points[0];
  let p2 = points[n - 1];

  if n == 2 {
    // Two points: the curve degenerates to a straight line when P₁ is the
    // midpoint of the chord. B(t) = (1-t)P₀ + tP₂.
    return ((p0 + p2) * T::half(), T::zero());
  }

  // Chord-length parameterization for points in this window
  let mut params = vec![T::zero(); n];
  let mut total_len = T::zero();
  for i in 1..n {
    total_len += (points[i] - points[i - 1]).length();
  }
  let degenerate_threshold: T = T::eval::<{ scalar_transmute(1e-12) }>();
  if total_len < degenerate_threshold {
    // Degenerate: all points coincident
    return (p0, T::zero());
  }
  let mut accum = T::zero();
  for i in 1..n {
    accum += (points[i] - points[i - 1]).length();
    params[i] = accum / total_len;
  }

  // Least squares: B(t) = (1-t)²P₀ + 2(1-t)t·P₁ + t²P₂
  // Define A(t) = 2(1-t)t, C(t) = (1-t)²P₀ + t²P₂
  // Then B(t) = A(t)·P₁ + C(t)
  // Minimize Σ|Qᵢ - A(tᵢ)P₁ - C(tᵢ)|² →
  //   P₁ = Σ A(tᵢ)(Qᵢ - C(tᵢ)) / Σ A(tᵢ)²
  let mut numerator = Vec2::new(T::zero(), T::zero());
  let mut denominator = T::zero();

  for i in 0..n {
    let t = params[i];
    let one_minus_t = T::one() - t;
    let a = T::two() * one_minus_t * t;
    let c = p0 * (one_minus_t * one_minus_t) + p2 * (t * t);
    numerator = numerator + (points[i] - c) * a;
    denominator += a * a;
  }

  let denom_threshold: T = T::eval::<{ scalar_transmute(1e-12) }>();
  let p1 = if denominator > denom_threshold {
    numerator / denominator
  } else {
    // Near-degenerate A(t) values (e.g. all points near endpoints):
    // fall back to a straight line
    (p0 + p2) * T::half()
  };

  // Compute maximum error across all points
  let mut max_err_sq = T::zero();
  for i in 0..n {
    let t = params[i];
    let one_minus_t = T::one() - t;
    let b = p0 * (one_minus_t * one_minus_t) + p1 * (T::two() * one_minus_t * t) + p2 * (t * t);
    let err_sq = (points[i] - b).length2();
    if err_sq > max_err_sq {
      max_err_sq = err_sq;
    }
  }

  (p1, max_err_sq)
}

/// Ray-casting point-in-polygon test for a closed 2D polygon.
///
/// Returns `true` if `point` is inside the polygon (including on edges).
/// The polygon is assumed closed (last point connects to first).
pub fn point_in_polygon(point: Vec2<f32>, polygon: &[Vec2<f32>]) -> bool {
  let n = polygon.len();
  if n < 3 {
    return false;
  }
  let mut inside = false;
  let mut j = n - 1;
  for i in 0..n {
    let a = polygon[i];
    let b = polygon[j];
    // Check if the ray from point to +x crosses edge (a,b)
    if (a.y > point.y) != (b.y > point.y) {
      let x_intersect = a.x + (b.x - a.x) * (point.y - a.y) / (b.y - a.y);
      if point.x < x_intersect {
        inside = !inside;
      }
    }
    j = i;
  }
  inside
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn point_in_square() {
    let square = vec![
      Vec2::new(0.0, 0.0),
      Vec2::new(1.0, 0.0),
      Vec2::new(1.0, 1.0),
      Vec2::new(0.0, 1.0),
    ];
    assert!(point_in_polygon(Vec2::new(0.5, 0.5), &square));
    assert!(point_in_polygon(Vec2::new(0.1, 0.1), &square));
    assert!(!point_in_polygon(Vec2::new(1.5, 0.5), &square));
    assert!(!point_in_polygon(Vec2::new(-0.1, 0.5), &square));
  }

  #[test]
  fn point_outside_polygon() {
    let triangle = vec![
      Vec2::new(0.0, 0.0),
      Vec2::new(1.0, 0.0),
      Vec2::new(0.5, 1.0),
    ];
    assert!(point_in_polygon(Vec2::new(0.5, 0.3), &triangle));
    assert!(!point_in_polygon(Vec2::new(0.5, -0.3), &triangle));
    assert!(!point_in_polygon(Vec2::new(1.5, 0.5), &triangle));
  }
}
