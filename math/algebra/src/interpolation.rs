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

pub trait Slerp<T> {
  fn slerp(self, rhs: Self, t: T) -> Self;
}
