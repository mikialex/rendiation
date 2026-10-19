use super::*;

/// Closest point on a Bézier curve: global search (subdivision / sampling) then Newton
/// refinement. With `guess`, a local search starting from that parameter is tried first.
/// Returns `(t, distance)`.
///
/// The global search samples the curve densely (relative to its degree), refines every sampled
/// local minimum with safeguarded Newton iterations on `(C(t) - p)·C'(t)` and keeps the best,
/// endpoints included. The local search refines inside a window of one sample spacing around
/// the guess; it is accepted when the minimum is found strictly inside that window (or on a
/// curve end), otherwise the global search runs.
pub fn closest_point_on_curve<P: HomogeneousPoint>(
  curve: &RationalBezierCurve<P>,
  point: P::Euclid,
  guess: Option<f64>,
) -> (f64, f64) {
  let distance = |t: f64| P::euclid_distance(curve.eval(t), point);
  if curve.control_points.len() == 1 {
    return (0., distance(0.));
  }
  let samples = (8 * curve.control_points.len()).max(32);
  let step = 1. / samples as f64;

  if let Some(guess) = guess.filter(|g| g.is_finite()) {
    let g = guess.clamp(0., 1.);
    let (lo, hi) = ((g - step).max(0.), (g + step).min(1.));
    let t = refine(curve, point, g, lo, hi);
    // Converging onto a window edge that is not a curve end means the minimum is outside.
    let margin = 1e-6 * step;
    if (t - lo > margin || lo == 0.) && (hi - t > margin || hi == 1.) {
      return (t, distance(t));
    }
  }

  let ts: Vec<_> = (0..=samples).map(|i| i as f64 * step).collect();
  let ds: Vec<_> = ts.iter().map(|t| distance(*t)).collect();
  let mut best = if ds[0] <= ds[samples] {
    (0., ds[0])
  } else {
    (1., ds[samples])
  };
  for i in 0..=samples {
    let left = if i > 0 { ds[i - 1] } else { f64::INFINITY };
    let right = if i < samples {
      ds[i + 1]
    } else {
      f64::INFINITY
    };
    if ds[i] <= left && ds[i] <= right {
      let lo = ts[i.saturating_sub(1)];
      let hi = ts[(i + 1).min(samples)];
      let t = refine(curve, point, ts[i], lo, hi);
      let d = distance(t);
      if d < best.1 {
        best = (t, d);
      }
    }
  }
  best
}

/// Closest point on a chain of curves. Returns `(segment index, t, distance)`.
pub fn closest_point_on_chain<P: HomogeneousPoint>(
  chain: &[RationalBezierCurve<P>],
  point: P::Euclid,
) -> Option<(usize, f64, f64)> {
  chain
    .iter()
    .enumerate()
    .map(|(i, c)| {
      let (t, d) = closest_point_on_curve(c, point, None);
      (i, t, d)
    })
    .min_by(|a, b| a.2.total_cmp(&b.2))
}

/// Minimize the distance inside `[lo, hi]` starting at `t`: Newton on `g(t) = (C - p)·C'`,
/// with the bracket shrunk by the sign of `g` and bisection whenever Newton leaves it. If the
/// minimum is on a bracket end the iteration converges to that end.
fn refine<P: HomogeneousPoint>(
  curve: &RationalBezierCurve<P>,
  point: P::Euclid,
  mut t: f64,
  mut lo: f64,
  mut hi: f64,
) -> f64 {
  for _ in 0..100 {
    let (c, d, dd) = curve.eval_with_second_derivative(t);
    let diff = c - point;
    let g = P::euclid_dot(diff, d);
    let gp = P::euclid_dot(d, d) + P::euclid_dot(diff, dd);
    if g > 0. {
      hi = t;
    } else if g < 0. {
      lo = t;
    } else {
      return t;
    }
    let newton = t - g / gp;
    let next = if gp > 0. && newton > lo && newton < hi {
      newton
    } else {
      0.5 * (lo + hi)
    };
    if (next - t).abs() <= 4. * f64::EPSILON || hi - lo <= 4. * f64::EPSILON {
      return next.clamp(lo, hi);
    }
    t = next;
  }
  t
}

#[cfg(test)]
mod tests {
  use std::f64::consts::FRAC_PI_2;

  use rendiation_algebra::*;

  use super::*;

  fn brute_force<P: HomogeneousPoint>(c: &RationalBezierCurve<P>, p: P::Euclid) -> f64 {
    (0..=20000)
      .map(|i| P::euclid_distance(c.eval(i as f64 / 20000.), p))
      .fold(f64::INFINITY, f64::min)
  }

  #[test]
  fn circle_projection() {
    let arc = elliptic_arc_2d(
      Vec2::new(0., 0.),
      Vec2::new(1., 0.),
      Vec2::new(0., 1.),
      2.,
      2.,
      0.,
      FRAC_PI_2,
    );
    for i in 0..=10 {
      let theta = FRAC_PI_2 * i as f64 / 10.;
      for r in [0.5, 2., 3.] {
        let p = Vec2::new(theta.cos(), theta.sin()) * r;
        let (t, d) = closest_point_on_curve(&arc, p, None);
        assert!(
          (t - arc_t_from_angle(theta, FRAC_PI_2)).abs() < 1e-10,
          "{theta} {r}"
        );
        assert!((d - (r - 2.).abs()).abs() < 1e-12);
        let (t2, _) = closest_point_on_curve(&arc, p, Some(0.5));
        assert!((t2 - t).abs() < 1e-10);
      }
    }
    // Beyond the ends.
    let (t, d) = closest_point_on_curve(&arc, Vec2::new(3., -1.), None);
    assert_eq!(t, 0.);
    assert!((d - 2f64.sqrt()).abs() < 1e-14);
    let (t, _) = closest_point_on_curve(&arc, Vec2::new(-1., 3.), Some(0.2));
    assert_eq!(t, 1.);
  }

  #[test]
  fn multiple_local_minima() {
    // S shaped cubic, the point is close to two branches.
    let c = RationalBezierCurve3::from_euclid(
      &[
        Vec3::new(0., 0., 0.),
        Vec3::new(3., 4., 0.),
        Vec3::new(-2., 4., 1.),
        Vec3::new(1., 0., 0.),
      ],
      Some(&[1., 0.5, 2., 1.]),
    );
    for i in 0..30 {
      let a = i as f64 * 0.37;
      let p = Vec3::new(a.cos() * 1.5 + 0.5, a.sin() * 1.5 + 1.5, (a * 2.).sin());
      let (t, d) = closest_point_on_curve(&c, p, None);
      assert!((P3::euclid_distance(c.eval(t), p) - d).abs() < 1e-14);
      assert!(d <= brute_force(&c, p) + 1e-12);
      // A guess near a worse local minimum still gives a local minimum (stationary).
      let (tg, _) = closest_point_on_curve(&c, p, Some(1. - t));
      if tg > 0. && tg < 1. {
        let (q, dq) = c.eval_with_derivative(tg);
        assert!((q - p).dot(dq).abs() < 1e-9 * (1. + dq.length()));
      }
    }
  }

  type P3 = Vec4<f64>;

  #[test]
  fn chain_projection() {
    let chain = vec![
      RationalBezierCurve2::line(Vec2::new(0., 0.), Vec2::new(1., 0.)),
      RationalBezierCurve2::line(Vec2::new(1., 0.), Vec2::new(1., 1.)),
      RationalBezierCurve2::line(Vec2::new(1., 1.), Vec2::new(0., 1.)),
    ];
    let (i, t, d) = closest_point_on_chain(&chain, Vec2::new(1.5, 0.25)).unwrap();
    assert_eq!(i, 1);
    assert!((t - 0.25).abs() < 1e-14);
    assert!((d - 0.5).abs() < 1e-14);
    let (i, t, _) = closest_point_on_chain(&chain, Vec2::new(0.3, 1.2)).unwrap();
    assert_eq!(i, 2);
    assert!((t - 0.7).abs() < 1e-14);
    assert!(closest_point_on_chain::<Vec3<f64>>(&[], Vec2::new(0., 0.)).is_none());
  }

  #[test]
  fn degenerate_curves() {
    let point = RationalBezierCurve2::new(vec![Vec3::new(1., 1., 1.)]);
    assert_eq!(
      closest_point_on_curve(&point, Vec2::new(1., 2.), None),
      (0., 1.)
    );
    // All control points equal: zero derivative everywhere.
    let flat = RationalBezierCurve2::new(vec![Vec3::new(1., 1., 1.); 3]);
    let (_, d) = closest_point_on_curve(&flat, Vec2::new(1., 2.), Some(0.5));
    assert!((d - 1.).abs() < 1e-15);
  }
}
