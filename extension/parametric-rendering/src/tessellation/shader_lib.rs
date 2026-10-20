//! Shader side building blocks of the GPU tessellation, mirroring the f64 CPU reference
//! (`parametric-tessellation`) operation by operation in f32: dyadic parameters, de Casteljau
//! evaluation and sub curve / sub patch extraction, the error bounds, iso line curves with the
//! canonical boundary direction, and the edge linearization queries.
//!
//! All functions emit inline code. Values that must be bitwise equal on both sides of a shared
//! edge (line curves, linearization decisions, boundary positions) are only computed from bitwise
//! equal inputs by the same code path of one pipeline.

use parametric_tessellation::{PARAM_BITS, PARAM_ONE};

use super::*;
use crate::*;

/// Control points of the longest curve (`MAX_GPU_DEGREE + 1`).
pub const MAX_CURVE_POINTS: usize = MAX_GPU_DEGREE + 1;
pub type CurvePoints = [Vec4<f32>; MAX_CURVE_POINTS];
pub type CurvePtr = ShaderPtrOf<CurvePoints>;

/// Stands in for infinite bounds (non positive weights), the Metal backend compiles with fast
/// math where infinities are not reliable.
pub const HUGE_BOUND: f32 = f32::MAX;

/// Reads the control point `(i, j)` of a patch.
pub type PatchReader<'a> = &'a dyn Fn(Node<u32>, Node<u32>) -> Node<Vec4<f32>>;

pub fn param_to_f32(n: Node<u32>) -> Node<f32> {
  n.into_f32() * val(1. / PARAM_ONE as f32)
}

/// The coarsest subdivision level at which `n` is a split point (0 for the domain ends).
pub fn shader_param_level(n: Node<u32>) -> Node<u32> {
  let interior = n.not_equals(val(0)).and(n.not_equals(val(PARAM_ONE)));
  interior.select(val(PARAM_BITS) - n.count_trailing_zeros(), val(0))
}

/// The dyadic interval `[index, index + 1] / 2^level`, see `parametric_tessellation::Segment`.
#[derive(Clone, Copy)]
pub struct ShaderSegment {
  pub level: Node<u32>,
  pub index: Node<u32>,
}

impl ShaderSegment {
  pub fn new(level: Node<u32>, index: Node<u32>) -> Self {
    Self { level, index }
  }

  pub fn start(self) -> Node<u32> {
    self.index << (val(PARAM_BITS) - self.level)
  }

  pub fn end(self) -> Node<u32> {
    (self.index + val(1)) << (val(PARAM_BITS) - self.level)
  }

  pub fn ancestor(self, level: Node<u32>) -> Self {
    Self {
      level,
      index: self.index >> (self.level - level),
    }
  }

  pub fn reversed(self) -> Self {
    Self {
      level: self.level,
      index: (val(1u32) << self.level) - val(1) - self.index,
    }
  }

  /// The segment of `level` containing `p` in its interior, the flag is false if `p` is a
  /// split point of that level (always for `level >= PARAM_BITS`).
  pub fn containing(level: Node<u32>, p: Node<u32>) -> (Node<bool>, Self) {
    let shift = val(PARAM_BITS) - level.min(val(PARAM_BITS));
    let mask = (val(1u32) << shift) - val(1);
    let segment = Self {
      level,
      index: p >> shift,
    };
    ((p & mask).not_equals(val(0)), segment)
  }
}

/// `a·(1-t) + b·t` like the CPU `blend`.
pub fn blend4(a: Node<Vec4<f32>>, b: Node<Vec4<f32>>, t: Node<f32>) -> Node<Vec4<f32>> {
  a * (val(1.) - t) + b * t
}

pub fn to_euclid(p: Node<Vec4<f32>>) -> Node<Vec3<f32>> {
  p.xyz() / p.w()
}

pub fn uvec4(a: Node<u32>, b: Node<u32>, c: Node<u32>, d: Node<u32>) -> Node<Vec4<u32>> {
  (a, b, c, d).into()
}

pub fn fvec4(a: Node<f32>, b: Node<f32>, c: Node<f32>, d: Node<f32>) -> Node<Vec4<f32>> {
  (a, b, c, d).into()
}

pub fn for_range(start: Node<u32>, end: Node<u32>, f: impl FnOnce(Node<u32>, &LoopCtx)) {
  (start..end).into_shader_iter().for_each(f)
}

pub fn make_curve_buffer() -> CurvePtr {
  make_local_var::<CurvePoints>()
}

pub fn copy_curve(from: &CurvePtr, to: &CurvePtr, degree: Node<u32>) {
  for_range(val(0), degree + val(1), |i, _| {
    to.index(i).store(from.index(i).load());
  });
}

/// de Casteljau evaluation of the `degree + 1` points in `buf`, destroys `buf`.
pub fn de_casteljau_in_place(buf: &CurvePtr, degree: Node<u32>, t: Node<f32>) -> Node<Vec4<f32>> {
  right_part_in_place(buf, degree, t);
  buf.index(val(0u32)).load()
}

/// Value and derivative like the CPU `de_casteljau_with_derivative`, destroys `buf`.
pub fn de_casteljau_with_derivative_in_place(
  buf: &CurvePtr,
  degree: Node<u32>,
  t: Node<f32>,
) -> (Node<Vec4<f32>>, Node<Vec4<f32>>) {
  let value = buf.index(val(0u32)).load().make_local_var();
  let derivative = zeroed_val::<Vec4<f32>>().make_local_var();
  if_by(degree.greater_than(val(0)), || {
    for_range(val(1), degree, |k, _| {
      for_range(val(0), degree + val(1) - k, |i, _| {
        let a = buf.index(i).load();
        let b = buf.index(i + val(1)).load();
        buf.index(i).store(blend4(a, b, t));
      });
    });
    let a = buf.index(val(0u32)).load();
    let b = buf.index(val(1u32)).load();
    value.store(blend4(a, b, t));
    derivative.store((b - a) * degree.into_f32());
  });
  (value.load(), derivative.load())
}

/// In place de Casteljau split keeping the part over `[0, t]`.
pub fn left_part_in_place(buf: &CurvePtr, degree: Node<u32>, t: Node<f32>) {
  for_range(val(1), degree + val(1), |k, _| {
    for_range(val(0), degree + val(1) - k, |jj, _| {
      let j = degree - jj;
      let a = buf.index(j - val(1)).load();
      let b = buf.index(j).load();
      buf.index(j).store(blend4(a, b, t));
    });
  });
}

/// In place de Casteljau split keeping the part over `[t, 1]`.
pub fn right_part_in_place(buf: &CurvePtr, degree: Node<u32>, t: Node<f32>) {
  for_range(val(1), degree + val(1), |k, _| {
    for_range(val(0), degree + val(1) - k, |i, _| {
      let a = buf.index(i).load();
      let b = buf.index(i + val(1)).load();
      buf.index(i).store(blend4(a, b, t));
    });
  });
}

/// The part over `[t0, t1]` (`t0 < t1`) reparameterized to `[0, 1]`, in place, with the same
/// split sequence as the CPU `sub_curve`.
pub fn sub_curve_in_place(buf: &CurvePtr, degree: Node<u32>, t0: Node<f32>, t1: Node<f32>) {
  if_by(t1.not_equals(val(1.)), || {
    left_part_in_place(buf, degree, t1)
  });
  if_by(t0.not_equals(val(0.)), || {
    let s = t1.equals(val(1.)).select(t0, t0 / t1);
    right_part_in_place(buf, degree, s);
  });
}

/// Blossom of the `degree + 1` points in `buf` at `t0` (`degree - i` times) and `t1` (`i`
/// times): control point `i` of the part over `[t0, t1]`. Destroys `buf`.
pub fn blossom(
  buf: &CurvePtr,
  degree: Node<u32>,
  i: Node<u32>,
  t0: Node<f32>,
  t1: Node<f32>,
) -> Node<Vec4<f32>> {
  for_range(val(1), degree + val(1), |l, _| {
    let t = l.less_equal_than(degree - i).select(t0, t1);
    for_range(val(0), degree + val(1) - l, |idx, _| {
      let a = buf.index(idx).load();
      let b = buf.index(idx + val(1)).load();
      buf.index(idx).store(blend4(a, b, t));
    });
  });
  buf.index(val(0u32)).load()
}

/// Control point `(i, j)` of the sub patch over `[u0, u1] × [v0, v1]`, computed on its own by
/// blossoming the rows, then the resulting column.
#[allow(clippy::too_many_arguments)]
pub fn sub_patch_point(
  cp: PatchReader,
  degree_u: Node<u32>,
  degree_v: Node<u32>,
  i: Node<u32>,
  j: Node<u32>,
  u: (Node<f32>, Node<f32>),
  v: (Node<f32>, Node<f32>),
) -> Node<Vec4<f32>> {
  let row = make_curve_buffer();
  let column = make_curve_buffer();
  for_range(val(0), degree_v + val(1), |r, _| {
    for_range(val(0), degree_u + val(1), |q, _| {
      row.index(q).store(cp(q, r));
    });
    column.index(r).store(blossom(&row, degree_u, i, u.0, u.1));
  });
  blossom(&column, degree_v, j, v.0, v.1)
}

/// Homogeneous surface point (rows first, like the CPU `eval_homogeneous`).
pub fn surface_eval(
  cp: PatchReader,
  degree_u: Node<u32>,
  degree_v: Node<u32>,
  u: Node<f32>,
  v: Node<f32>,
) -> Node<Vec4<f32>> {
  let row = make_curve_buffer();
  let column = make_curve_buffer();
  for_range(val(0), degree_v + val(1), |r, _| {
    for_range(val(0), degree_u + val(1), |q, _| {
      row.index(q).store(cp(q, r));
    });
    column
      .index(r)
      .store(de_casteljau_in_place(&row, degree_u, u));
  });
  de_casteljau_in_place(&column, degree_v, v)
}

/// Euclidean point, ∂S/∂u, ∂S/∂v like the CPU `eval_with_partials`.
pub fn surface_eval_with_partials(
  cp: PatchReader,
  degree_u: Node<u32>,
  degree_v: Node<u32>,
  u: Node<f32>,
  v: Node<f32>,
) -> (Node<Vec3<f32>>, Node<Vec3<f32>>, Node<Vec3<f32>>) {
  let row = make_curve_buffer();
  let column = make_curve_buffer();
  let column_du = make_curve_buffer();
  for_range(val(0), degree_v + val(1), |r, _| {
    for_range(val(0), degree_u + val(1), |q, _| {
      row.index(q).store(cp(q, r));
    });
    let (value, du) = de_casteljau_with_derivative_in_place(&row, degree_u, u);
    column.index(r).store(value);
    column_du.index(r).store(du);
  });
  let (h, h_v) = de_casteljau_with_derivative_in_place(&column, degree_v, v);
  let h_u = de_casteljau_in_place(&column_du, degree_v, v);
  let p = to_euclid(h);
  let inv_w = val(1.) / h.w();
  let du = (h_u.xyz() - p * h_u.w()) * inv_w;
  let dv = (h_v.xyz() - p * h_v.w()) * inv_w;
  (p, du, dv)
}

/// Weight of coefficient `i - a` of a degree `n` Bernstein polynomial in coefficient `i` of its
/// product with a degree 1 polynomial's coefficient `a` (CPU `elevation_factor`).
fn elevation_factor(n: Node<u32>, i: Node<u32>, a: u32) -> Node<f32> {
  let numerator = if a == 0 { n + val(1) - i } else { i };
  numerator.into_f32() / (n + val(1)).into_f32()
}

fn second_diff(a: Node<Vec3<f32>>, b: Node<Vec3<f32>>, c: Node<Vec3<f32>>) -> Node<Vec3<f32>> {
  a - b - b + c
}

fn second_diff_w(a: Node<f32>, b: Node<f32>, c: Node<f32>) -> Node<f32> {
  a - b - b + c
}

fn max_by(slot: &ShaderPtrOf<f32>, value: Node<f32>) {
  slot.store(slot.load().max(value));
}

/// `parametric_tessellation::CurveBounds` on the shader side.
#[derive(Clone, Copy)]
pub struct ShaderCurveBounds {
  pub min: Node<Vec3<f32>>,
  pub max: Node<Vec3<f32>>,
  pub chord_error: Node<f32>,
  pub polyline_coefficient: Node<f32>,
}

impl ShaderCurveBounds {
  pub fn polyline_error(&self, m: Node<u32>) -> Node<f32> {
    let second = self.polyline_coefficient / (m * m).into_f32();
    m.equals(val(1))
      .select(second.min(self.chord_error), second)
  }
}

/// Port of the CPU `curve_bounds` for the `degree + 1` points in `buf`.
pub fn shader_curve_bounds(buf: &CurvePtr, degree: Node<u32>) -> ShaderCurveBounds {
  let n = degree;
  let first = to_euclid(buf.index(val(0u32)).load());
  let min = first.make_local_var();
  let max = first.make_local_var();
  let w_min = val(HUGE_BOUND).make_local_var();
  for_range(val(0), n + val(1), |i, _| {
    let p = buf.index(i).load();
    let e = to_euclid(p);
    min.store(min.load().min(e));
    max.store(max.load().max(e));
    w_min.store(w_min.load().min(p.w()));
  });
  let (min, max, w_min) = (min.load(), max.load(), w_min.load());

  let chord = val(0.).make_local_var();
  let polyline = val(HUGE_BOUND).make_local_var();
  let chord_error = val(HUGE_BOUND).make_local_var();
  if_by(w_min.greater_than(val(0.)), || {
    let ends = [
      to_euclid(buf.index(val(0u32)).load()),
      to_euclid(buf.index(n).load()),
    ];
    for_range(val(0), n + val(2), |i, _| {
      let acc = zeroed_val::<Vec3<f32>>().make_local_var();
      if_by(i.less_equal_than(n), || {
        let p = buf.index(i).load();
        acc.store(acc.load() + (p.xyz() - ends[0] * p.w()) * elevation_factor(n, i, 0));
      });
      if_by(i.greater_equal_than(val(1)), || {
        let p = buf.index(i - val(1)).load();
        acc.store(acc.load() + (p.xyz() - ends[1] * p.w()) * elevation_factor(n, i, 1));
      });
      max_by(&chord, acc.load().length());
    });

    let center = (min + max) * val(0.5);
    let g = |i: Node<u32>| {
      let p = buf.index(i).load();
      p.xyz() - center * p.w()
    };
    let w = |i: Node<u32>| buf.index(i).load().w();
    let radius = val(0.).make_local_var();
    let g1 = val(0.).make_local_var();
    let d1 = val(0.).make_local_var();
    let g2 = val(0.).make_local_var();
    let d2 = val(0.).make_local_var();
    for_range(val(0), n + val(1), |i, _| {
      max_by(&radius, (to_euclid(buf.index(i).load()) - center).length());
      if_by(i.greater_equal_than(val(1)), || {
        max_by(&g1, (g(i) - g(i - val(1))).length());
        max_by(&d1, (w(i) - w(i - val(1))).abs());
      });
      if_by(i.greater_equal_than(val(2)), || {
        max_by(
          &g2,
          second_diff(g(i - val(2)), g(i - val(1)), g(i)).length(),
        );
        max_by(&d2, second_diff_w(w(i - val(2)), w(i - val(1)), w(i)).abs());
      });
    });
    let nf = n.into_f32();
    let g1 = nf * g1.load();
    let d1 = nf * d1.load();
    let g2 = nf * (nf - val(1.)) * g2.load();
    let d2 = nf * (nf - val(1.)) * d2.load();
    let radius = radius.load();
    let s1 = (g1 + d1 * radius) / w_min;
    let m2 = (g2 + val(2.) * d1 * s1 + d2 * radius) / w_min;
    chord_error.store(chord.load() / w_min);
    polyline.store(m2 / val(8.));
  });

  ShaderCurveBounds {
    min,
    max,
    chord_error: chord_error.load(),
    polyline_coefficient: polyline.load(),
  }
}

/// `parametric_tessellation::SurfaceBounds` on the shader side.
#[derive(Clone, Copy)]
pub struct ShaderSurfaceBounds {
  pub min: Node<Vec3<f32>>,
  pub max: Node<Vec3<f32>>,
  pub quad_error: Node<f32>,
  pub grid_coefficient: Node<f32>,
}

impl ShaderSurfaceBounds {
  pub fn grid_error(&self, k: Node<u32>) -> Node<f32> {
    let second = self.grid_coefficient / (k * k).into_f32();
    k.equals(val(1)).select(second.min(self.quad_error), second)
  }
}

/// Port of the CPU `surface_bounds`.
pub fn shader_surface_bounds(cp: PatchReader, n: Node<u32>, m: Node<u32>) -> ShaderSurfaceBounds {
  let first = to_euclid(cp(val(0), val(0)));
  let min = first.make_local_var();
  let max = first.make_local_var();
  let w_min = val(HUGE_BOUND).make_local_var();
  for_range(val(0), m + val(1), |j, _| {
    for_range(val(0), n + val(1), |i, _| {
      let p = cp(i, j);
      let e = to_euclid(p);
      min.store(min.load().min(e));
      max.store(max.load().max(e));
      w_min.store(w_min.load().min(p.w()));
    });
  });
  let (min, max, w_min) = (min.load(), max.load(), w_min.load());

  let quad_error = val(HUGE_BOUND).make_local_var();
  let grid_coefficient = val(HUGE_BOUND).make_local_var();
  if_by(w_min.greater_than(val(0.)), || {
    let corners = [
      to_euclid(cp(val(0), val(0))),
      to_euclid(cp(n, val(0))),
      to_euclid(cp(val(0), m)),
      to_euclid(cp(n, m)),
    ];
    let bilinear = val(0.).make_local_var();
    for_range(val(0), m + val(2), |j, _| {
      for_range(val(0), n + val(2), |i, _| {
        let acc = zeroed_val::<Vec3<f32>>().make_local_var();
        for (corner, (a, b)) in [(0, 0), (1, 0), (0, 1), (1, 1)].into_iter().enumerate() {
          let valid = i
            .greater_equal_than(val(a))
            .and(i.less_equal_than(n + val(a)))
            .and(j.greater_equal_than(val(b)))
            .and(j.less_equal_than(m + val(b)));
          if_by(valid, || {
            let p = cp(i - val(a), j - val(b));
            let factor = elevation_factor(n, i, a) * elevation_factor(m, j, b);
            acc.store(acc.load() + (p.xyz() - corners[corner] * p.w()) * factor);
          });
        }
        max_by(&bilinear, acc.load().length());
      });
    });
    let twist = (corners[0] - corners[1] - corners[2] + corners[3]).length() * val(0.25);

    let center = (min + max) * val(0.5);
    let g = |i: Node<u32>, j: Node<u32>| {
      let p = cp(i, j);
      p.xyz() - center * p.w()
    };
    let w = |i: Node<u32>, j: Node<u32>| cp(i, j).w();
    let one = val(1u32);
    let two = val(2u32);
    let maxima: [ShaderPtrOf<f32>; 11] = std::array::from_fn(|_| val(0.).make_local_var());
    let [radius, gu, gv, du, dv, guu, gvv, duu, dvv, guv, duv] = &maxima;
    for_range(val(0), m + val(1), |j, _| {
      for_range(val(0), n + val(1), |i, _| {
        max_by(radius, (to_euclid(cp(i, j)) - center).length());
        if_by(i.greater_equal_than(one), || {
          max_by(gu, (g(i, j) - g(i - one, j)).length());
          max_by(du, (w(i, j) - w(i - one, j)).abs());
        });
        if_by(j.greater_equal_than(one), || {
          max_by(gv, (g(i, j) - g(i, j - one)).length());
          max_by(dv, (w(i, j) - w(i, j - one)).abs());
        });
        if_by(i.greater_equal_than(two), || {
          max_by(
            guu,
            second_diff(g(i - two, j), g(i - one, j), g(i, j)).length(),
          );
          max_by(
            duu,
            second_diff_w(w(i - two, j), w(i - one, j), w(i, j)).abs(),
          );
        });
        if_by(j.greater_equal_than(two), || {
          max_by(
            gvv,
            second_diff(g(i, j - two), g(i, j - one), g(i, j)).length(),
          );
          max_by(
            dvv,
            second_diff_w(w(i, j - two), w(i, j - one), w(i, j)).abs(),
          );
        });
        if_by(
          i.greater_equal_than(one).and(j.greater_equal_than(one)),
          || {
            let twist_g = g(i, j) - g(i - one, j) - g(i, j - one) + g(i - one, j - one);
            max_by(guv, twist_g.length());
            let twist_w = w(i, j) - w(i - one, j) - w(i, j - one) + w(i - one, j - one);
            max_by(duv, twist_w.abs());
          },
        );
      });
    });
    let (nf, mf) = (n.into_f32(), m.into_f32());
    let gu = nf * gu.load();
    let gv = mf * gv.load();
    let du = nf * du.load();
    let dv = mf * dv.load();
    let guu = nf * (nf - val(1.)) * guu.load();
    let gvv = mf * (mf - val(1.)) * gvv.load();
    let duu = nf * (nf - val(1.)) * duu.load();
    let dvv = mf * (mf - val(1.)) * dvv.load();
    let guv = nf * mf * guv.load();
    let duv = nf * mf * duv.load();
    let radius = radius.load();

    let su = (gu + du * radius) / w_min;
    let sv = (gv + dv * radius) / w_min;
    let muu = (guu + val(2.) * du * su + duu * radius) / w_min;
    let mvv = (gvv + val(2.) * dv * sv + dvv * radius) / w_min;
    let muv = (guv + du * sv + dv * su + duv * radius) / w_min;
    quad_error.store(bilinear.load() / w_min + twist);
    grid_coefficient.store((muu + val(2.) * muv + mvv) / val(8.));
  });

  ShaderSurfaceBounds {
    min,
    max,
    quad_error: quad_error.load(),
    grid_coefficient: grid_coefficient.load(),
  }
}

/// The frustum test of the CPU `Camera::culls`.
pub fn culls(u: &ENode<TessUniform>, min: Node<Vec3<f32>>, max: Node<Vec3<f32>>) -> Node<bool> {
  let far_depth = -min.z();
  let x = u.frustum_x * far_depth;
  let y = u.frustum_y * far_depth;
  far_depth
    .less_than(u.near)
    .or(min.x().greater_than(x))
    .or(max.x().less_than(-x))
    .or(min.y().greater_than(y))
    .or(max.y().less_than(-y))
}

/// An f32 key ordered like `f32::total_cmp`.
fn total_order_key(x: Node<f32>) -> Node<u32> {
  let bits = x.bitcast::<u32>();
  let negative = (bits & val(0x8000_0000)).not_equals(val(0));
  negative.select(bits.bitwise_not(), bits | val(0x8000_0000))
}

/// An iso line curve in `points`, see the CPU `LineCurve`.
#[derive(Clone, Copy)]
pub struct ShaderLineCurve {
  pub degree: Node<u32>,
  /// The canonical direction is the reverse of the patch parameter direction.
  pub reversed: Node<bool>,
  /// The subdivision level where the line first becomes a sub patch edge.
  pub level: Node<u32>,
}

impl ShaderLineCurve {
  /// Patch parameter along the line to canonical parameter, also the inverse.
  pub fn canonical(&self, p: Node<u32>) -> Node<u32> {
    self.reversed.select(val(PARAM_ONE) - p, p)
  }

  pub fn canonical_segment(&self, s: ShaderSegment) -> ShaderSegment {
    let r = s.reversed();
    ShaderSegment {
      level: s.level,
      index: self.reversed.select(r.index, s.index),
    }
  }
}

/// Loads the iso line `u = coord` (`axis_v` false, parameterized by v) or `v = coord` (`axis_v`
/// true) of the patch into `points`. Patch boundary lines are taken from the boundary control
/// points and put in the canonical direction: the lexicographically smaller control point
/// sequence (by `f32::total_cmp` of x, y, z, w) first. Interior lines are de Casteljau
/// evaluated across, keeping the patch direction. `tmp` is scratch space.
pub fn load_line_curve(
  cp: PatchReader,
  degree_u: Node<u32>,
  degree_v: Node<u32>,
  axis_v: Node<bool>,
  coord: Node<u32>,
  points: &CurvePtr,
  tmp: &CurvePtr,
) -> ShaderLineCurve {
  let degree = axis_v.select(degree_u, degree_v);
  let across = axis_v.select(degree_v, degree_u);
  // Point `s` of the line comes from strip `s`, a row (u = coord) or column (v = coord).
  let strip_point = |s: Node<u32>, q: Node<u32>| {
    let (i, j) = (axis_v.select(s, q), axis_v.select(q, s));
    cp(i, j)
  };
  let boundary = coord.equals(val(0)).or(coord.equals(val(PARAM_ONE)));
  let t = param_to_f32(coord);
  for_range(val(0), degree + val(1), |s, _| {
    if_by(boundary, || {
      let q = coord.equals(val(0)).select(val(0), across);
      points.index(s).store(strip_point(s, q));
    })
    .else_by(|| {
      for_range(val(0), across + val(1), |q, _| {
        tmp.index(q).store(strip_point(s, q));
      });
      points.index(s).store(de_casteljau_in_place(tmp, across, t));
    });
  });

  let reversed = val(false).make_local_var();
  if_by(boundary, || {
    let decided = val(false).make_local_var();
    for_range(val(0), degree + val(1), |idx, cx| {
      let a = points.index(degree - idx).load();
      let b = points.index(idx).load();
      for c in 0..4 {
        if_by(decided.load().not(), || {
          let (ka, kb) = (total_order_key(a.index(c)), total_order_key(b.index(c)));
          if_by(ka.not_equals(kb), || {
            decided.store(val(true));
            reversed.store(ka.less_than(kb));
          });
        });
      }
      if_by(decided.load(), || cx.do_break());
    });
    if_by(reversed.load(), || {
      for_range(val(0), (degree + val(1)) / val(2), |i, _| {
        let a = points.index(i).load();
        let b = points.index(degree - i).load();
        points.index(i).store(b);
        points.index(degree - i).store(a);
      });
    });
  });

  ShaderLineCurve {
    degree,
    reversed: reversed.load(),
    level: shader_param_level(coord),
  }
}

/// Number of uniform pieces linearizing the canonical `segment` of the line within the edge
/// tolerance, 0 if more than `k_max` are needed (segments at `max_level` always succeed). Like
/// the CPU `linearization`, a pure function of the line curve and the segment.
pub fn linearization(
  u: &ENode<TessUniform>,
  curve: &CurvePtr,
  degree: Node<u32>,
  segment: ShaderSegment,
  tmp: &CurvePtr,
) -> Node<u32> {
  copy_curve(curve, tmp, degree);
  sub_curve_in_place(
    tmp,
    degree,
    param_to_f32(segment.start()),
    param_to_f32(segment.end()),
  );
  let bounds = shader_curve_bounds(tmp, degree);
  let depth = -bounds.max.z();
  let pieces = val(0u32).make_local_var();
  if_by(depth.greater_than(u.near), || {
    let tolerance = u.edge_tolerance * depth / u.focal_length;
    for_range(val(0), u.log2_k_max + val(1), |e, cx| {
      let m = val(1u32) << e;
      if_by(bounds.polyline_error(m).less_equal_than(tolerance), || {
        pieces.store(m);
        cx.do_break();
      });
    });
  });
  let pieces = pieces.load();
  pieces
    .equals(val(0))
    .and(segment.level.greater_equal_than(u.max_level))
    .select(u.k_max, pieces)
}

/// The coarsest linearizable segment containing the canonical `segment`, searched from the
/// line's root level down, with its piece count (0 if none).
pub fn flat_ancestor(
  u: &ENode<TessUniform>,
  curve: &CurvePtr,
  line: ShaderLineCurve,
  segment: ShaderSegment,
  tmp: &CurvePtr,
) -> (ShaderSegment, Node<u32>) {
  let level = val(0u32).make_local_var();
  let pieces = val(0u32).make_local_var();
  for_range(line.level, segment.level + val(1), |l, cx| {
    let m = linearization(u, curve, line.degree, segment.ancestor(l), tmp);
    if_by(m.not_equals(val(0)), || {
      level.store(l);
      pieces.store(m);
      cx.do_break();
    });
  });
  (segment.ancestor(level.load()), pieces.load())
}

/// The linearization piece containing canonical parameter `p` in its interior, the flag is
/// false if `p` is a break point (or no linearized segment contains it in its interior).
pub fn locate(
  u: &ENode<TessUniform>,
  curve: &CurvePtr,
  line: ShaderLineCurve,
  p: Node<u32>,
  tmp: &CurvePtr,
) -> (Node<bool>, ShaderSegment) {
  let found = val(false).make_local_var();
  let piece_level = val(0u32).make_local_var();
  let piece_index = val(0u32).make_local_var();
  let (inside, root) = ShaderSegment::containing(line.level, p);
  if_by(inside, || {
    let level = root.level.make_local_var();
    let index = root.index.make_local_var();
    loop_by(|cx| {
      let segment = ShaderSegment::new(level.load(), index.load());
      let m = linearization(u, curve, line.degree, segment, tmp);
      if_by(m.not_equals(val(0)), || {
        let (inside, piece) =
          ShaderSegment::containing(segment.level + m.count_trailing_zeros(), p);
        found.store(inside);
        piece_level.store(piece.level);
        piece_index.store(piece.index);
        cx.do_break();
      });
      let (inside, next) = ShaderSegment::containing(segment.level + val(1), p);
      if_by(inside.not(), || cx.do_break());
      level.store(next.level);
      index.store(next.index);
    });
  });
  (
    found.load(),
    ShaderSegment::new(piece_level.load(), piece_index.load()),
  )
}
