//! Conservative approximation error bounds of rational Bézier patches and curves against their
//! piecewise linear interpolation in parameter space (`|S(u,v) - T(u,v)|` at the same
//! parameter, which also bounds the geometric distance).
//!
//! Two bounds are used, both valid for positive weights:
//!
//! Bilinear: with `B` the bilinear interpolation of the corners, `S - B = (H - B·w) / w` where
//! `H` is the homogeneous surface (`x·w, y·w, z·w`) and `w` its weight. The numerator is a
//! polynomial of degree `(n+1, m+1)` whose Bernstein coefficients bound it, `w >= min w_ij`.
//! For polynomial patches this is the control net against bilinear criterion of Eisenacher
//! 2009 (after one degree elevation). The two triangles differ from `B` by a quarter of the
//! twist vector.
//!
//! Second derivative: linear interpolation over the triangulated `k×k` grid has error at most
//! `(M_uu + 2·M_uv + M_vv) / (8·k²)` (Filip, Magedson, Markot 1986, via Cauchy Schwarz for the
//! mixed term). For rational patches the derivative bounds come from the quotient rule on
//! `S - X = G / w`, `G = H - w·X`, with `X` the control box center.

use parametric_normalized_model::*;
use rendiation_algebra::*;

#[derive(Debug, Clone, Copy)]
pub struct SurfaceBounds {
  /// Box of the Euclidean control points, it contains the surface.
  pub min: Vec3<f64>,
  pub max: Vec3<f64>,
  /// Error bound of the two triangle quad over the whole domain.
  pub quad_error: f64,
  /// `c` in the `c / k²` error bound of the triangulated `k×k` grid.
  pub grid_coefficient: f64,
}

impl SurfaceBounds {
  pub fn grid_error(&self, k: u32) -> f64 {
    let second = self.grid_coefficient / (k * k) as f64;
    if k == 1 {
      second.min(self.quad_error)
    } else {
      second
    }
  }
}

#[derive(Debug, Clone, Copy)]
pub struct CurveBounds {
  pub min: Vec3<f64>,
  pub max: Vec3<f64>,
  /// Error bound of the chord.
  pub chord_error: f64,
  /// `c` in the `c / m²` error bound of the uniform `m` piece polyline.
  pub polyline_coefficient: f64,
}

impl CurveBounds {
  pub fn polyline_error(&self, m: u32) -> f64 {
    let second = self.polyline_coefficient / (m * m) as f64;
    if m == 1 {
      second.min(self.chord_error)
    } else {
      second
    }
  }
}

pub fn surface_bounds(s: &RationalBezierSurface) -> SurfaceBounds {
  let (n, m) = (s.degree_u, s.degree_v);
  let h = |i: usize, j: usize| s.control_point(i, j);
  let (min, max) = euclid_box(&s.control_points);
  let w_min = min_weight(&s.control_points);
  if w_min.is_nan() || w_min <= 0. {
    return SurfaceBounds {
      min,
      max,
      quad_error: f64::INFINITY,
      grid_coefficient: f64::INFINITY,
    };
  }

  let corners = [
    h(0, 0).to_euclid(),
    h(n, 0).to_euclid(),
    h(0, m).to_euclid(),
    h(n, m).to_euclid(),
  ];
  let mut bilinear: f64 = 0.;
  for j in 0..=m + 1 {
    for i in 0..=n + 1 {
      let mut acc = Vec3::new(0., 0., 0.);
      for (corner, (a, b)) in [(0, 0), (1, 0), (0, 1), (1, 1)].into_iter().enumerate() {
        if i < a || i - a > n || j < b || j - b > m {
          continue;
        }
        let p = h(i - a, j - b);
        let factor = elevation_factor(n, i, a) * elevation_factor(m, j, b);
        acc += (p.vector_part() - corners[corner] * p.w) * factor;
      }
      bilinear = bilinear.max(acc.length());
    }
  }
  let twist = (corners[0] - corners[1] - corners[2] + corners[3]).length() * 0.25;

  let center = (min + max) * 0.5;
  let g = |i: usize, j: usize| {
    let p = h(i, j);
    p.vector_part() - center * p.w
  };
  let w = |i: usize, j: usize| h(i, j).w;
  let radius = max_over(0..=n, 0..=m, |i, j| (h(i, j).to_euclid() - center).length());
  let (nf, mf) = (n as f64, m as f64);
  let gu = nf * max_over(1..=n, 0..=m, |i, j| (g(i, j) - g(i - 1, j)).length());
  let gv = mf * max_over(0..=n, 1..=m, |i, j| (g(i, j) - g(i, j - 1)).length());
  let du = nf * max_over(1..=n, 0..=m, |i, j| (w(i, j) - w(i - 1, j)).abs());
  let dv = mf * max_over(0..=n, 1..=m, |i, j| (w(i, j) - w(i, j - 1)).abs());
  let guu = nf
    * (nf - 1.)
    * max_over(2..=n, 0..=m, |i, j| {
      second_diff(g(i - 2, j), g(i - 1, j), g(i, j)).length()
    });
  let gvv = mf
    * (mf - 1.)
    * max_over(0..=n, 2..=m, |i, j| {
      second_diff(g(i, j - 2), g(i, j - 1), g(i, j)).length()
    });
  let duu = nf
    * (nf - 1.)
    * max_over(2..=n, 0..=m, |i, j| {
      second_diff(w(i - 2, j), w(i - 1, j), w(i, j)).abs()
    });
  let dvv = mf
    * (mf - 1.)
    * max_over(0..=n, 2..=m, |i, j| {
      second_diff(w(i, j - 2), w(i, j - 1), w(i, j)).abs()
    });
  let guv = nf
    * mf
    * max_over(1..=n, 1..=m, |i, j| {
      (g(i, j) - g(i - 1, j) - g(i, j - 1) + g(i - 1, j - 1)).length()
    });
  let duv = nf
    * mf
    * max_over(1..=n, 1..=m, |i, j| {
      (w(i, j) - w(i - 1, j) - w(i, j - 1) + w(i - 1, j - 1)).abs()
    });

  let su = (gu + du * radius) / w_min;
  let sv = (gv + dv * radius) / w_min;
  let muu = (guu + 2. * du * su + duu * radius) / w_min;
  let mvv = (gvv + 2. * dv * sv + dvv * radius) / w_min;
  let muv = (guv + du * sv + dv * su + duv * radius) / w_min;

  SurfaceBounds {
    min,
    max,
    quad_error: bilinear / w_min + twist,
    grid_coefficient: (muu + 2. * muv + mvv) / 8.,
  }
}

pub fn curve_bounds(c: &RationalBezierCurve3) -> CurveBounds {
  let cp = &c.control_points;
  let n = cp.len() - 1;
  let (min, max) = euclid_box(cp);
  let w_min = min_weight(cp);
  if w_min.is_nan() || w_min <= 0. {
    return CurveBounds {
      min,
      max,
      chord_error: f64::INFINITY,
      polyline_coefficient: f64::INFINITY,
    };
  }

  let ends = [cp[0].to_euclid(), cp[n].to_euclid()];
  let mut chord: f64 = 0.;
  for i in 0..=n + 1 {
    let mut acc = Vec3::new(0., 0., 0.);
    for (a, end) in ends.iter().enumerate() {
      if i < a || i - a > n {
        continue;
      }
      let p = cp[i - a];
      acc += (p.vector_part() - *end * p.w) * elevation_factor(n, i, a);
    }
    chord = chord.max(acc.length());
  }

  let center = (min + max) * 0.5;
  let g = |i: usize| cp[i].vector_part() - center * cp[i].w;
  let w = |i: usize| cp[i].w;
  let radius = max_over(0..=n, 0..=0, |i, _| (cp[i].to_euclid() - center).length());
  let nf = n as f64;
  let g1 = nf * max_over(1..=n, 0..=0, |i, _| (g(i) - g(i - 1)).length());
  let d1 = nf * max_over(1..=n, 0..=0, |i, _| (w(i) - w(i - 1)).abs());
  let g2 = nf
    * (nf - 1.)
    * max_over(2..=n, 0..=0, |i, _| {
      second_diff(g(i - 2), g(i - 1), g(i)).length()
    });
  let d2 = nf
    * (nf - 1.)
    * max_over(2..=n, 0..=0, |i, _| {
      second_diff(w(i - 2), w(i - 1), w(i)).abs()
    });
  let s1 = (g1 + d1 * radius) / w_min;
  let m2 = (g2 + 2. * d1 * s1 + d2 * radius) / w_min;

  CurveBounds {
    min,
    max,
    chord_error: chord / w_min,
    polyline_coefficient: m2 / 8.,
  }
}

/// Weight of coefficient `i - a` of a degree `n` Bernstein polynomial in coefficient `i` of its
/// product with a degree 1 polynomial's coefficient `a` (also the degree elevation weights).
fn elevation_factor(n: usize, i: usize, a: usize) -> f64 {
  let numerator = if a == 0 { n + 1 - i } else { i };
  numerator as f64 / (n + 1) as f64
}

fn second_diff<T: Blend>(a: T, b: T, c: T) -> T {
  a - b - b + c
}

fn max_over(
  is: std::ops::RangeInclusive<usize>,
  js: std::ops::RangeInclusive<usize>,
  f: impl Fn(usize, usize) -> f64,
) -> f64 {
  let mut result: f64 = 0.;
  for j in js {
    for i in is.clone() {
      result = result.max(f(i, j));
    }
  }
  result
}

fn euclid_box(cp: &[Vec4<f64>]) -> (Vec3<f64>, Vec3<f64>) {
  let first = cp[0].to_euclid();
  cp.iter()
    .map(|p| p.to_euclid())
    .fold((first, first), |(min, max), p| (min.min(p), max.max(p)))
}

fn min_weight(cp: &[Vec4<f64>]) -> f64 {
  cp.iter().map(|p| p.w).fold(f64::INFINITY, f64::min)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::tests::*;

  /// Dense samples of the triangulated grid against the surface stay within the bounds.
  #[test]
  fn surface_bounds_hold() {
    let mut rng = TestRng(7);
    for case in 0..200 {
      let s = random_surface(&mut rng, case % 3 == 0);
      let b = surface_bounds(&s);
      for k in [1, 2, 4] {
        let measured = sampled_grid_error(&s, k);
        assert!(
          measured <= b.grid_error(k) * (1. + 1e-9) + 1e-12,
          "case {case} k {k}: measured {measured} bound {}",
          b.grid_error(k)
        );
      }
    }
  }

  #[test]
  fn curve_bounds_hold() {
    let mut rng = TestRng(11);
    for case in 0..200 {
      let s = random_surface(&mut rng, case % 2 == 0);
      let c = s.iso_curve(Axis::U, rng.next());
      let b = curve_bounds(&c);
      for m in [1, 2, 4, 8] {
        let mut measured: f64 = 0.;
        for piece in 0..m {
          let (t0, t1) = (piece as f64 / m as f64, (piece + 1) as f64 / m as f64);
          let (a, e) = (c.eval(t0), c.eval(t1));
          for i in 0..=32 {
            let x = i as f64 / 32.;
            let p = c.eval(t0 + (t1 - t0) * x);
            measured = measured.max((p - blend(a, e, x)).length());
          }
        }
        assert!(measured <= b.polyline_error(m) * (1. + 1e-9) + 1e-12);
      }
    }
  }

  fn sampled_grid_error(s: &RationalBezierSurface, k: u32) -> f64 {
    let kf = k as f64;
    let mut result: f64 = 0.;
    for cj in 0..k {
      for ci in 0..k {
        let corner = |di: u32, dj: u32| {
          let uv = ((ci + di) as f64 / kf, (cj + dj) as f64 / kf);
          (uv, s.eval(uv.0, uv.1))
        };
        let (p00, p10, p01, p11) = (corner(0, 0), corner(1, 0), corner(0, 1), corner(1, 1));
        for tri in [[p00, p10, p11], [p00, p11, p01]] {
          let r = 12;
          for a in 0..=r {
            for b in 0..=r - a {
              let l = [
                a as f64 / r as f64,
                b as f64 / r as f64,
                (r - a - b) as f64 / r as f64,
              ];
              let u = tri[0].0.0 * l[0] + tri[1].0.0 * l[1] + tri[2].0.0 * l[2];
              let v = tri[0].0.1 * l[0] + tri[1].0.1 * l[1] + tri[2].0.1 * l[2];
              let t = tri[0].1 * l[0] + tri[1].1 * l[1] + tri[2].1 * l[2];
              result = result.max((s.eval(u, v) - t).length());
            }
          }
        }
      }
    }
    result
  }
}
