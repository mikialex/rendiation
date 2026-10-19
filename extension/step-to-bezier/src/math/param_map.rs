use super::*;

/// How one segment of a `PiecewiseParamMap` maps native parameter to the local Bézier
/// parameter `t ∈ [0, 1]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentMap {
  /// `t = (x - b_i) / (b_{i+1} - b_i)`.
  Affine,
  /// The segment is an angle interval of span at most π/2 represented by a rational quadratic
  /// arc: `t = arc_t_from_angle(x - b_i, b_{i+1} - b_i)`.
  CircularArc,
}

/// Monotone map from a native parameter axis to P space. Native breakpoints
/// `breaks[0] < breaks[1] < ... < breaks[n]`, segment `i` covers `[breaks[i], breaks[i+1]]`
/// and maps to P interval `[i, i+1]`.
///
/// If `period` is set, `breaks[n] - breaks[0] == period` and the map extends periodically:
/// `to_p(x + period) == to_p(x) + n`. Without a period, values outside the breaks extrapolate
/// with the first / last segment map.
#[derive(Debug, Clone, PartialEq)]
pub struct PiecewiseParamMap {
  pub breaks: Vec<f64>,
  pub segments: Vec<SegmentMap>,
  pub period: Option<f64>,
}

impl PiecewiseParamMap {
  pub fn new(breaks: Vec<f64>, segments: Vec<SegmentMap>, period: Option<f64>) -> Self {
    assert_eq!(breaks.len(), segments.len() + 1);
    Self {
      breaks,
      segments,
      period,
    }
  }

  pub fn single_affine(x0: f64, x1: f64) -> Self {
    Self::new(vec![x0, x1], vec![SegmentMap::Affine], None)
  }

  pub fn segment_count(&self) -> usize {
    self.segments.len()
  }

  /// Period in P units (the segment count) when the map is periodic.
  pub fn p_period(&self) -> Option<f64> {
    self.period.map(|_| self.segments.len() as f64)
  }

  pub fn is_affine(&self) -> bool {
    self.segments.iter().all(|s| *s == SegmentMap::Affine)
  }

  /// Native value to P coordinate. Values within a relative `1e-13` of a breakpoint (in the
  /// segment's local parameter) snap exactly onto the integer P coordinate of the break.
  pub fn to_p(&self, x: f64) -> f64 {
    let (k, i, local) = self.locate(x);
    let n = self.segments.len() as f64;
    let mut t = self.local_t(i, local);
    if t.abs() <= SNAP {
      t = 0.;
    } else if (t - 1.).abs() <= SNAP {
      t = 1.;
    }
    k as f64 * n + i as f64 + t
  }

  pub fn to_native(&self, p: f64) -> f64 {
    let n = self.segments.len();
    let (k, local_p) = match self.period {
      Some(_) => {
        let k = (p / n as f64).floor();
        (k, p - k * n as f64)
      }
      None => (0., p),
    };
    let i = (local_p.floor().max(0.) as usize).min(n - 1);
    let t = local_p - i as f64;
    let (b0, b1) = (self.breaks[i], self.breaks[i + 1]);
    let local = match self.segments[i] {
      _ if t == 0. => b0,
      _ if t == 1. => b1,
      SegmentMap::Affine => b0 * (1. - t) + b1 * t,
      SegmentMap::CircularArc => b0 + arc_angle_from_t(t, b1 - b0),
    };
    match self.period {
      Some(period) => k * period + local,
      None => local,
    }
  }

  /// Native breakpoints strictly inside `(x0, x1)` including periodic repetitions, ascending.
  /// Used to split curves before mapping them segment by segment. The bounds may be given in
  /// any order. Periodic repetitions are `k·period + breaks[i]`, the same expression
  /// `to_native` produces for integer P values. Ranges covering more than `MAX_PERIODS`
  /// periods are rejected (empty result), they can only come from corrupt data.
  pub fn breaks_between(&self, x0: f64, x1: f64) -> Vec<f64> {
    let (lo, hi) = (x0.min(x1), x0.max(x1));
    if !(lo.is_finite() && hi.is_finite()) {
      return Vec::new();
    }
    let n = self.segments.len();
    match self.period {
      None => self
        .breaks
        .iter()
        .copied()
        .filter(|b| lo < *b && *b < hi)
        .collect(),
      Some(period) => {
        let b0 = self.breaks[0];
        let k0 = ((lo - b0) / period).floor() - 1.;
        let k1 = ((hi - b0) / period).ceil() + 1.;
        if k1 - k0 > MAX_PERIODS {
          return Vec::new();
        }
        let mut out = Vec::new();
        let mut k = k0;
        while k <= k1 {
          for b in &self.breaks[..n] {
            let v = k * period + b;
            if lo < v && v < hi {
              out.push(v);
            }
          }
          k += 1.;
        }
        out
      }
    }
  }

  /// For native value `x` (periodic aware): segment index in P (may be negative or beyond the
  /// segment count for periodic repetitions), and the affine coefficients `(scale, offset)` so
  /// that `p = x * scale + offset` within that segment, or `None` for non affine segments.
  pub fn segment_affine(&self, x: f64) -> (i64, Option<(f64, f64)>) {
    let (k, i, _) = self.locate(x);
    let index = k * self.segments.len() as i64 + i as i64;
    let affine = match self.segments[i] {
      SegmentMap::CircularArc => None,
      SegmentMap::Affine => {
        let (b0, b1) = (self.breaks[i], self.breaks[i + 1]);
        let scale = 1. / (b1 - b0);
        let shift = k as f64 * self.period.unwrap_or(0.);
        Some((scale, index as f64 - (shift + b0) * scale))
      }
    };
    (index, affine)
  }

  /// Period index `k`, segment `i` and the value reduced into the base period (`x - k·period`).
  /// Without a period `k` is 0 and out of range values use the first / last segment.
  fn locate(&self, x: f64) -> (i64, usize, f64) {
    let n = self.segments.len();
    let (k, local) = match self.period {
      Some(period) => {
        let k = ((x - self.breaks[0]) / period).floor();
        if !k.is_finite() {
          return (0, 0, f64::NAN);
        }
        (k as i64, x - k * period)
      }
      None => (0, x),
    };
    let i = self
      .breaks
      .partition_point(|b| *b <= local)
      .saturating_sub(1)
      .min(n - 1);
    (k, i, local)
  }

  fn local_t(&self, i: usize, x: f64) -> f64 {
    let (b0, b1) = (self.breaks[i], self.breaks[i + 1]);
    match self.segments[i] {
      SegmentMap::Affine => (x - b0) / (b1 - b0),
      SegmentMap::CircularArc => arc_t_from_angle(x - b0, b1 - b0),
    }
  }
}

const SNAP: f64 = 1e-13;
const MAX_PERIODS: f64 = 1e6;

#[cfg(test)]
mod tests {
  use std::f64::consts::{FRAC_PI_2, PI, TAU};

  use super::*;

  fn periodic_arc_map() -> PiecewiseParamMap {
    let a0 = 0.4;
    let breaks = (0..=4).map(|i| a0 + FRAC_PI_2 * i as f64).collect();
    let mut map = PiecewiseParamMap::new(breaks, vec![SegmentMap::CircularArc; 4], Some(TAU));
    map.breaks[4] = a0 + TAU;
    map
  }

  fn mixed_map() -> PiecewiseParamMap {
    PiecewiseParamMap::new(
      vec![-1., 0.5, 1.2, 3.],
      vec![
        SegmentMap::Affine,
        SegmentMap::CircularArc,
        SegmentMap::Affine,
      ],
      None,
    )
  }

  fn periodic_affine_map() -> PiecewiseParamMap {
    PiecewiseParamMap::new(vec![2., 2.5, 4., 5.], vec![SegmentMap::Affine; 3], Some(3.))
  }

  #[test]
  fn affine_basics() {
    let m = PiecewiseParamMap::single_affine(2., 6.);
    assert!(m.is_affine());
    assert_eq!(m.to_p(2.), 0.);
    assert_eq!(m.to_p(6.), 1.);
    assert_eq!(m.to_p(4.), 0.5);
    assert_eq!(m.to_p(10.), 2.);
    assert_eq!(m.to_p(0.), -0.5);
    assert_eq!(m.to_native(-0.5), 0.);
    assert_eq!(m.to_native(1.), 6.);
    assert_eq!(m.p_period(), None);
    assert!(!mixed_map().is_affine());
  }

  #[test]
  fn breaks_map_to_integers() {
    for map in [periodic_arc_map(), mixed_map(), periodic_affine_map()] {
      for (i, b) in map.breaks.iter().enumerate() {
        assert_eq!(map.to_p(*b), i as f64);
        assert_eq!(map.to_native(i as f64), *b);
      }
    }
  }

  #[test]
  fn round_trips() {
    for map in [periodic_arc_map(), mixed_map(), periodic_affine_map()] {
      for i in -300..=300 {
        let x = i as f64 * 0.037 + 0.001;
        let p = map.to_p(x);
        assert!((map.to_native(p) - x).abs() < 1e-12 * (1. + x.abs()), "{x}");
        let p = i as f64 * 0.013;
        assert!(
          (map.to_p(map.to_native(p)) - p).abs() < 1e-12 * (1. + p.abs()),
          "{p}"
        );
      }
    }
  }

  #[test]
  fn monotone() {
    for map in [periodic_arc_map(), mixed_map(), periodic_affine_map()] {
      let mut prev = f64::NEG_INFINITY;
      for i in -200..=200 {
        let p = map.to_p(i as f64 * 0.01);
        assert!(p > prev);
        prev = p;
      }
    }
  }

  #[test]
  fn periodic_extension() {
    for map in [periodic_arc_map(), periodic_affine_map()] {
      let period = map.period.unwrap();
      let n = map.p_period().unwrap();
      for k in [-7i32, -2, -1, 1, 3, 25] {
        for i in 0..40 {
          let x = map.breaks[0] + period * i as f64 / 40. + 0.0123;
          let shifted = map.to_p(x + period * k as f64);
          assert!(
            (shifted - (map.to_p(x) + n * k as f64)).abs() < 1e-11,
            "{k} {x}"
          );
          let p = map.to_p(x);
          let back = map.to_native(p + n * k as f64);
          assert!((back - (x + period * k as f64)).abs() < 1e-11);
        }
      }
    }
  }

  #[test]
  fn arc_segment_matches_arc_parameter() {
    let map = mixed_map();
    let span = 0.7;
    for i in 0..=10 {
      let theta = span * i as f64 / 10.;
      let expected = 1. + arc_t_from_angle(theta, span);
      assert!((map.to_p(0.5 + theta) - expected).abs() < 1e-13);
    }
  }

  #[test]
  fn breaks_between_ranges() {
    let map = mixed_map();
    assert_eq!(map.breaks_between(-5., 5.), vec![-1., 0.5, 1.2, 3.]);
    assert_eq!(map.breaks_between(0.5, 3.), vec![1.2]);
    assert_eq!(map.breaks_between(3., 0.), vec![0.5, 1.2]);
    assert!(map.breaks_between(1.3, 2.9).is_empty());

    let map = periodic_affine_map();
    assert_eq!(map.breaks_between(2., 5.), vec![2.5, 4.]);
    assert_eq!(map.breaks_between(-1.5, 2.5), vec![-1., -0.5, 1., 2.]);
    let many = map.breaks_between(-30.1, 30.1);
    assert_eq!(many.len(), 3 * 20);
    assert!(many.windows(2).all(|w| w[0] < w[1]));
    for b in many {
      let p = map.to_p(b);
      assert_eq!(p, p.round(), "{b}");
    }

    let map = periodic_arc_map();
    let b = map.breaks_between(-10., 10.);
    assert!(
      b.windows(2)
        .all(|w| (w[1] - w[0] - FRAC_PI_2).abs() < 1e-12)
    );
    for x in b {
      let p = map.to_p(x);
      assert_eq!(p, p.round());
      assert_eq!(map.to_native(p), x);
    }
  }

  #[test]
  fn segment_affine_coefficients() {
    let map = periodic_affine_map();
    for i in -100..100 {
      let x = i as f64 * 0.173 + 0.01;
      let (index, coeff) = map.segment_affine(x);
      let (scale, offset) = coeff.unwrap();
      let p = map.to_p(x);
      assert!((x * scale + offset - p).abs() < 1e-11, "{x}");
      assert_eq!(index, p.floor() as i64);
    }
    let map = mixed_map();
    assert_eq!(map.segment_affine(0.7), (1, None));
    let (index, coeff) = map.segment_affine(-3.);
    assert_eq!(index, 0);
    let (scale, offset) = coeff.unwrap();
    assert!((-3. * scale + offset - map.to_p(-3.)).abs() < 1e-14);
    let (index, coeff) = map.segment_affine(PI + 10.);
    assert_eq!(index, 2);
    let (scale, offset) = coeff.unwrap();
    assert!(((PI + 10.) * scale + offset - map.to_p(PI + 10.)).abs() < 1e-13);
  }
}
