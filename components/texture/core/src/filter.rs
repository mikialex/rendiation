use rendiation_algebra::{Lerp, Scalar};

/// Texel mixing mode when sampling between texels.
pub trait TextureFilterMode<T: Scalar, P> {
  fn interpolate(t: T, one: P, other: P) -> P;
}

pub struct Nearest;
impl<T: Scalar, P> TextureFilterMode<T, P> for Nearest {
  fn interpolate(t: T, one: P, other: P) -> P {
    if t > T::half() {
      other
    } else {
      one
    }
  }
}

pub struct Linear;
impl<T: Scalar, P: Lerp<T>> TextureFilterMode<T, P> for Linear {
  fn interpolate(t: T, one: P, other: P) -> P {
    one.lerp(other, t)
  }
}
