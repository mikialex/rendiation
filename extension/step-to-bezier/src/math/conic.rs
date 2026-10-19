use rendiation_algebra::*;

use super::*;

/// Split `[a0, a1]` (radians, `a1 > a0`) into the minimal number of equal sub ranges of span at
/// most `max_span`. The first range starts exactly at `a0` and the last ends exactly at `a1`.
/// Returns an empty list when the range is empty or not finite.
pub fn split_angle_range(a0: f64, a1: f64, max_span: f64) -> Vec<(f64, f64)> {
  let span = a1 - a0;
  if !(span.is_finite() && span > 0. && max_span > 0.) {
    return Vec::new();
  }
  // The relative slack keeps spans that are an exact multiple of `max_span` (up to rounding,
  // for example a full turn in quarter arcs) from getting one extra piece.
  let n = ((span / max_span) * (1. - 1e-12)).ceil().max(1.) as usize;
  let at = |i: usize| {
    if i == 0 {
      a0
    } else if i == n {
      a1
    } else {
      a0 + span * (i as f64 / n as f64)
    }
  };
  (0..n).map(|i| (at(i), at(i + 1))).collect()
}

/// Exact rational quadratic for the elliptic arc
/// `center + rx·cos(θ)·x_axis + ry·sin(θ)·y_axis`, `θ ∈ [a0, a1]`, `0 < a1 - a0 <= π/2`.
/// `x_axis` and `y_axis` are orthonormal. The curve parameter relates to the angle through
/// `arc_t_from_angle` / `arc_angle_from_t` with span `a1 - a0`.
pub fn elliptic_arc_3d(
  center: Vec3<f64>,
  x_axis: Vec3<f64>,
  y_axis: Vec3<f64>,
  rx: f64,
  ry: f64,
  a0: f64,
  a1: f64,
) -> RationalBezierCurve3 {
  let point = |c: f64, s: f64| center + x_axis * (rx * c) + y_axis * (ry * s);
  let (p0, p1, p2, w) = arc_control(a0, a1, point);
  RationalBezierCurve::new(vec![
    Vec4::from_euclid(p0, 1.),
    Vec4::from_euclid(p1, w),
    Vec4::from_euclid(p2, 1.),
  ])
}

pub fn elliptic_arc_2d(
  center: Vec2<f64>,
  x_axis: Vec2<f64>,
  y_axis: Vec2<f64>,
  rx: f64,
  ry: f64,
  a0: f64,
  a1: f64,
) -> RationalBezierCurve2 {
  let point = |c: f64, s: f64| center + x_axis * (rx * c) + y_axis * (ry * s);
  let (p0, p1, p2, w) = arc_control(a0, a1, point);
  RationalBezierCurve::new(vec![
    Vec3::from_euclid(p0, 1.),
    Vec3::from_euclid(p1, w),
    Vec3::from_euclid(p2, 1.),
  ])
}

/// Euclidean control points and middle weight of the arc. The middle point is the intersection
/// of the end tangents: the mid angle direction scaled by `1 / cos(span / 2)` (in the affine
/// image of the unit circle).
fn arc_control<T>(a0: f64, a1: f64, point: impl Fn(f64, f64) -> T) -> (T, T, T, f64) {
  let half = 0.5 * (a1 - a0);
  let mid = a0 + half;
  let w = half.cos();
  let p0 = point(a0.cos(), a0.sin());
  let p1 = point(mid.cos() / w, mid.sin() / w);
  let p2 = point(a1.cos(), a1.sin());
  (p0, p1, p2, w)
}

/// For a rational quadratic arc of angular span `span` built by `elliptic_arc_*`, the curve
/// parameter `t` at local angle `theta ∈ [0, span]`:
/// `t = 0.5 + tan((theta - span/2)/2) / (2·tan(span/4))`.
///
/// Values of `theta` outside `[0, span]` extrapolate (monotone while
/// `|theta - span/2| < π`). A zero span returns 0.
pub fn arc_t_from_angle(theta: f64, span: f64) -> f64 {
  if span == 0. {
    return 0.;
  }
  0.5 + (0.5 * (theta - 0.5 * span)).tan() / (2. * (0.25 * span).tan())
}

/// Inverse of `arc_t_from_angle`: `theta = span/2 + 2·atan((2t - 1)·tan(span/4))`.
pub fn arc_angle_from_t(t: f64, span: f64) -> f64 {
  0.5 * span + 2. * ((2. * t - 1.) * (0.25 * span).tan()).atan()
}

#[cfg(test)]
mod tests {
  use std::f64::consts::{FRAC_PI_2, PI, TAU};

  use super::*;

  #[test]
  fn split_ranges() {
    let r = split_angle_range(0., TAU, FRAC_PI_2);
    assert_eq!(r.len(), 4);
    assert_eq!(r[0].0, 0.);
    assert_eq!(r[3].1, TAU);
    for w in r.windows(2) {
      assert_eq!(w[0].1, w[1].0);
    }
    let r = split_angle_range(0.3, 0.3 + PI + 0.01, FRAC_PI_2);
    assert_eq!(r.len(), 3);
    let spans: Vec<_> = r.iter().map(|(a, b)| b - a).collect();
    for s in &spans {
      assert!(*s <= FRAC_PI_2 && (s - spans[0]).abs() < 1e-14);
    }
    assert_eq!(split_angle_range(1., 1.2, FRAC_PI_2), vec![(1., 1.2)]);
    assert!(split_angle_range(1., 1., FRAC_PI_2).is_empty());
    assert!(split_angle_range(1., f64::NAN, FRAC_PI_2).is_empty());
    // Exactly three quarters, computed with rounding.
    assert_eq!(
      split_angle_range(-0.1, -0.1 + 3. * FRAC_PI_2, FRAC_PI_2).len(),
      3
    );
  }

  fn check_arc(rx: f64, ry: f64, a0: f64, a1: f64) {
    let center = Vec3::new(1., -2., 0.5);
    let x = Vec3::new(1., 1., 0.).normalize();
    let y = Vec3::new(-1., 1., 1.).normalize();
    let y = (y - x * y.dot(x)).normalize();
    let arc = elliptic_arc_3d(center, x, y, rx, ry, a0, a1);
    let span = a1 - a0;
    for i in 0..=64 {
      let t = i as f64 / 64.;
      let p = arc.eval(t) - center;
      let (cx, cy) = (p.dot(x) / rx, p.dot(y) / ry);
      // On the ellipse.
      assert!((cx * cx + cy * cy - 1.).abs() < 1e-13, "{cx} {cy}");
      // Parameter / angle relation.
      let mut theta = cy.atan2(cx) - a0;
      theta -= TAU * ((theta + PI) / TAU).floor();
      let expected = arc_angle_from_t(t, span);
      assert!((theta - expected).abs() < 1e-12, "{theta} {expected}");
      assert!((arc_t_from_angle(theta, span) - t).abs() < 1e-12);
    }
    let start = center + x * (rx * a0.cos()) + y * (ry * a0.sin());
    let end = center + x * (rx * a1.cos()) + y * (ry * a1.sin());
    assert!((arc.start() - start).length() < 1e-15 * (1. + start.length()));
    assert!((arc.end() - end).length() < 1e-15 * (1. + end.length()));

    let arc2 = elliptic_arc_2d(
      Vec2::new(0.5, 0.25),
      Vec2::new(0., 1.),
      Vec2::new(-1., 0.),
      rx,
      ry,
      a0,
      a1,
    );
    for i in 0..=16 {
      let t = i as f64 / 16.;
      let p = arc2.eval(t) - Vec2::new(0.5, 0.25);
      let theta = arc_angle_from_t(t, span) + a0;
      let expected =
        Vec2::new(0., 1.) * (rx * theta.cos()) + Vec2::new(-1., 0.) * (ry * theta.sin());
      assert!((p - expected).length() < 1e-13);
    }
  }

  #[test]
  fn arcs_are_exact() {
    check_arc(1., 1., 0., FRAC_PI_2);
    check_arc(2.5, 0.75, 0.3, 0.3 + FRAC_PI_2);
    check_arc(3., 1., -2., -1.9);
    check_arc(1., 2., 1., 1.0 + 1e-3);
    check_arc(1., 1., 4., 4. + PI / 3.);
  }

  #[test]
  fn angle_param_round_trip() {
    for span in [FRAC_PI_2, 1.0, 0.1, 1e-4] {
      assert_eq!(arc_t_from_angle(0., span), 0.);
      assert!((arc_t_from_angle(span, span) - 1.).abs() < 1e-15);
      assert!((arc_angle_from_t(1., span) - span).abs() < 1e-15);
      assert!(arc_angle_from_t(0., span).abs() < 1e-15);
      let mut prev = -1.;
      for i in -10..=74 {
        let t = i as f64 / 64.;
        let theta = arc_angle_from_t(t, span);
        assert!(theta > prev);
        prev = theta;
        assert!((arc_t_from_angle(theta, span) - t).abs() < 1e-12);
      }
    }
    assert_eq!(arc_t_from_angle(0.3, 0.), 0.);
  }
}
