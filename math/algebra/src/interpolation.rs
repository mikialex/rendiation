/// Linear interpolation, `t = 0` returns self, `t = 1` returns rhs. `t` is not clamped.
pub trait Lerp<T> {
  #[must_use]
  fn lerp(self, rhs: Self, t: T) -> Self;
}

impl Lerp<f32> for f32 {
  #[inline(always)]
  fn lerp(self, b: Self, t: f32) -> Self {
    self * (1.0 - t) + b * t
  }
}

impl Lerp<f64> for f64 {
  #[inline(always)]
  fn lerp(self, b: Self, t: Self) -> Self {
    self * (1.0 - t) + b * t
  }
}

/// Spherical linear interpolation, `t = 0` returns self, `t = 1` returns rhs.
pub trait Slerp<T> {
  #[must_use]
  fn slerp(self, rhs: Self, t: T) -> Self;
}

#[test]
fn scalar_lerp() {
  assert_eq!(1.0_f32.lerp(3.0, 0.0), 1.0);
  assert_eq!(1.0_f32.lerp(3.0, 1.0), 3.0);
  assert_eq!(1.0_f64.lerp(3.0, 0.5), 2.0);
  assert_eq!(1.0_f64.lerp(3.0, 2.0), 5.0);
}
