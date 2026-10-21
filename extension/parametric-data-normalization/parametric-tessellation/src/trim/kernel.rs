use std::ops::{Add, Div, Mul, Sub};

use rendiation_algebra::*;

use crate::trim::*;

/// Samples are clamped to `[UV_CLAMP_MIN, UV_CLAMP_MAX]²` (`2⁻²⁰` away from the domain sides),
/// so they never lie on a segment on the domain boundary: a sample at `u = 1` would miss the
/// boundary segments on `u = 1` (they must lie strictly right of the ray origin), and the root
/// code treats a ray exactly at a segment end point v as passing below it, so a sample at
/// `v = 0` would miss the segments starting on `v = 0`.
pub const UV_CLAMP_MIN: f32 = 1. / 1048576.;
pub const UV_CLAMP_MAX: f32 = 1. - 1. / 1048576.;

pub fn clamp_uv(uv: Vec2<f32>) -> Vec2<f32> {
  Vec2::new(
    uv.x.clamp(UV_CLAMP_MIN, UV_CLAMP_MAX),
    uv.y.clamp(UV_CLAMP_MIN, UV_CLAMP_MAX),
  )
}

/// Signed crossing count of the `+u` ray from `uv` with the segment, f32 operation by operation
/// like the shader. Positive for a segment crossing the ray upwards (increasing v), so a point
/// inside a counter clockwise loop gets `+1` (the opposite sign of Slug's `xcov`).
pub fn crossing(segment: &EncodedSegment, uv: Vec2<f32>) -> i32 {
  crossing_generic(
    [
      [segment.p0.x, segment.p0.y],
      [segment.p1.x, segment.p1.y],
      [segment.p2.x, segment.p2.y],
    ],
    segment.max_u,
    [uv.x, uv.y],
  )
}

/// The same in f64, used as the reference for the f32 rounding.
pub fn crossing_f64(segment: &TrimQuadratic, uv: Vec2<f64>) -> i32 {
  let [p0, p1, p2] = segment.points;
  crossing_generic(
    [[p0.x, p0.y], [p1.x, p1.y], [p2.x, p2.y]],
    p0.x.max(p1.x).max(p2.x),
    [uv.x, uv.y],
  )
}

/// Slug's root eligibility code from the sign bits of the sample relative control point v
/// coordinates: bit 0 set if root `t1` is a crossing of the segment, bit 8 for root `t2`. It
/// depends on signs only, so a ray through the shared end point of two segments, or touching
/// a segment at a horizontal tangent, is counted consistently without any epsilon.
pub fn root_code(negative0: bool, negative1: bool, negative2: bool) -> u32 {
  let shift = negative0 as u32 | (negative1 as u32) << 1 | (negative2 as u32) << 2;
  (0x2E74 >> shift) & 0x0101
}

/// Segments with all control points left of the sample contribute nothing, this is part of the
/// definition (not only an optimization) so that a band scan with early exit and a scan of all
/// segments agree bitwise.
///
/// Roots of `a t² - 2b t + c = 0` (`a = p0 - 2p1 + p2`, `b = p0 - p1`, `c = p0`) are computed in
/// the cancellation free form, `t1 = c / (b + d)` for `b ≥ 0` and `t2 = c / (b - d)` for `b ≤ 0`,
/// instead of Slug's linear branch below `|a| < 1/65536`: trim segments in `[0, 1]²` are short,
/// so their `a` is regularly below that fixed threshold. Without a real crossing (discriminant
/// `≤ 0`, both roots eligible because a control point is past the ray) both roots are the double
/// root `b / a` like in Slug, so their contributions cancel.
fn crossing_generic<T: KernelScalar>(points: [[T; 2]; 3], max_u: T, uv: [T; 2]) -> i32 {
  if max_u < uv[0] {
    return 0;
  }
  let rel = |p: [T; 2]| [p[0] - uv[0], p[1] - uv[1]];
  let (p0, p1, p2) = (rel(points[0]), rel(points[1]), rel(points[2]));
  let code = root_code(
    p0[1].sign_negative(),
    p1[1].sign_negative(),
    p2[1].sign_negative(),
  );
  if code == 0 {
    return 0;
  }
  let two = T::from_f64(2.);
  let zero = T::from_f64(0.);
  let a = [p0[0] - p1[0] * two + p2[0], p0[1] - p1[1] * two + p2[1]];
  let b = [p0[0] - p1[0], p0[1] - p1[1]];
  let discriminant = b[1] * b[1] - a[1] * p0[1];
  let (t1, t2) = if discriminant > zero {
    let d = discriminant.sqrt();
    let t1 = if b[1] >= zero {
      p0[1] / (b[1] + d)
    } else {
      (b[1] - d) / a[1]
    };
    let t2 = if b[1] <= zero {
      p0[1] / (b[1] - d)
    } else {
      (b[1] + d) / a[1]
    };
    (t1, t2)
  } else {
    let t = if a[1] != zero { b[1] / a[1] } else { zero };
    (t, t)
  };
  let x_at = |t: T| (a[0] * t - b[0] * two) * t + p0[0];

  let mut winding = 0;
  if code & 1 != 0 && x_at(t1) > zero {
    winding -= 1;
  }
  if code > 1 && x_at(t2) > zero {
    winding += 1;
  }
  winding
}

trait KernelScalar:
  Copy
  + PartialOrd
  + Add<Output = Self>
  + Sub<Output = Self>
  + Mul<Output = Self>
  + Div<Output = Self>
{
  fn from_f64(v: f64) -> Self;
  fn sign_negative(self) -> bool;
  fn sqrt(self) -> Self;
}

impl KernelScalar for f32 {
  fn from_f64(v: f64) -> Self {
    v as f32
  }
  fn sign_negative(self) -> bool {
    self.to_bits() >> 31 != 0
  }
  fn sqrt(self) -> Self {
    f32::sqrt(self)
  }
}

impl KernelScalar for f64 {
  fn from_f64(v: f64) -> Self {
    v
  }
  fn sign_negative(self) -> bool {
    self.to_bits() >> 63 != 0
  }
  fn sqrt(self) -> Self {
    f64::sqrt(self)
  }
}
