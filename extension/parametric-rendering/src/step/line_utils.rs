use crate::*;

// ContinuousTrimPolyline, but verify loop closed
#[derive(Debug)]
pub struct CompletedTrimPolyline {
  polyline: ContinuousTrimPolyline,
  /// Signed area: > 0 for CCW (outer), < 0 for CW (hole).
  signed_area: f32,
}

impl CompletedTrimPolyline {
  /// Returns `None` if the line has <= 2 points or is not closed.
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

    let area = compute_signed_area_inner(&line);
    Some(Self {
      polyline: line,
      signed_area: area,
    })
  }

  pub fn signed_area(&self) -> f32 {
    self.signed_area
  }

  pub fn iter_no_edge_polylines(&self) -> impl Iterator<Item = &NoEdgeContinuousTrimPolyline> {
    self.polyline.polylines.iter()
  }

  pub fn iter_points(&self) -> impl Iterator<Item = Vec2<f32>> + '_ {
    self
      .polyline
      .polylines
      .iter()
      .enumerate()
      .flat_map(|(i, poly)| {
        let start = if i == 0 { 0 } else { 1 };
        poly.points[start..].iter().copied()
      })
  }

  pub fn reconstruct_quadratic_bezier_curves(
    self,
    error_distance_tolerance: f32,
  ) -> Vec<QuadraticBezierCurve2d<f32>> {
    self
      .polyline
      .reconstruct_quadratic_bezier_curves(error_distance_tolerance)
  }
}

fn compute_signed_area_inner(line: &ContinuousTrimPolyline) -> f32 {
  let mut points = Vec::new();
  for (i, poly) in line.polylines.iter().enumerate() {
    if i == 0 {
      points.extend_from_slice(&poly.points);
    } else {
      points.extend_from_slice(&poly.points[1..]);
    }
  }
  if points.len() < 3 {
    return 0.0;
  }
  if let Some(&first) = points.first() {
    points.push(first);
  }
  let mut area = 0.0;
  for i in 0..points.len() - 1 {
    area += points[i].x * points[i + 1].y - points[i + 1].x * points[i].y;
  }
  area * 0.5
}

/// A single continuous(maybe contains edges) point polyline in UV space.
/// (will be reconstructed by one of more bezier curves)
///
/// the topology is line-strip
#[derive(Default, Debug)]
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
    assert!(points.len() >= 2);
    let polylines = points
      .array_windows::<2>()
      .map(|[a, b]| NoEdgeContinuousTrimPolyline::line_segment(*a, *b))
      .collect();
    Self { polylines }
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
      self.push_no_edge_polyline(polyline, true);
    }
  }

  pub fn push_no_edge_polyline(
    &mut self,
    mut polyline: NoEdgeContinuousTrimPolyline,
    fix_periodic_jump: bool,
  ) {
    assert!(!polyline.is_degenerate());
    if let Some(last) = self.polylines.last() {
      let old_last = *last.points.last().unwrap();
      let new_start = *polyline.points.first().unwrap();

      if fix_periodic_jump {
        let du = new_start.x - old_last.x;
        let dv = new_start.y - old_last.y;
        if du.abs() > 0.99 {
          for p in &mut polyline.points {
            p.x -= du.round();
          }
        } else if dv.abs() > 0.99 {
          for p in &mut polyline.points {
            p.y -= dv.round();
          }
        }
      }

      let new_start = *polyline.points.first().unwrap();
      assert!(
        old_last.distance_to(new_start) <= 1e-6,
        "polyline discontinuity: old_last={old_last:?}, new_start={new_start:?}, dist={}",
        old_last.distance_to(new_start)
      );
    }

    self.polylines.push(polyline);
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
#[derive(Default, Debug)]
pub struct NoEdgeContinuousTrimPolyline {
  points: Vec<Vec2<f32>>,
}

impl NoEdgeContinuousTrimPolyline {
  pub fn new_assume_no_edge(points: Vec<Vec2<f32>>) -> Self {
    Self { points }
  }

  pub fn iter_points(&self) -> impl Iterator<Item = Vec2<f32>> + '_ {
    self.points.iter().copied()
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

/// True winding number of `point` with respect to a set of trim loops.
///
/// Outer (CCW) loops contribute +1, hole (CW) loops contribute -1.
/// The winding number sums directly over all directed edges using the
/// standard point-in-polygon winding algorithm, so loop orientation is
/// handled intrinsically — no separate signed-area pre-classification
/// is needed. Result > 0 means the point lies inside the face.
pub fn compute_winding_number(point: Vec2<f32>, loops: &[CompletedTrimPolyline]) -> i32 {
  let mut wn: i32 = 0;
  for lp in loops {
    let points: Vec<Vec2<f32>> = lp.iter_points().collect();
    if points.len() < 2 {
      continue;
    }
    for i in 0..points.len() - 1 {
      let a = points[i];
      let b = points[i + 1];
      if a.y <= point.y {
        if b.y > point.y && is_left(a, b, point) > 0.0 {
          wn += 1;
        }
      } else {
        if b.y <= point.y && is_left(a, b, point) < 0.0 {
          wn -= 1;
        }
      }
    }
  }
  wn
}

/// Cross product (b - a) × (p - a): positive when p is left of the
/// directed edge a → b.
fn is_left(a: Vec2<f32>, b: Vec2<f32>, p: Vec2<f32>) -> f32 {
  (b.x - a.x) * (p.y - a.y) - (p.x - a.x) * (b.y - a.y)
}
