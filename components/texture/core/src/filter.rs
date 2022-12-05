use rendiation_algebra::{Lerp, Scalar};

/// Texel mixing mode when sampling between texels.
pub trait TextureFilterMode<T: Scalar, P> {
  const ENUM: FilterMode;
  fn interpolate(t: T, one: P, other: P) -> P;
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub enum FilterMode {
  Nearest,
  Linear,
}

incremental::clone_self_incremental!(FilterMode);

impl FilterMode {
  pub fn interpolate<T, P>(&self, t: T, one: P, other: P) -> P
  where
    T: Scalar,
    P: Lerp<T>,
  {
    match self {
      Self::Nearest => Nearest::interpolate(t, one, other),
      Self::Linear => Linear::interpolate(t, one, other),
    }
  }
}

pub struct Nearest;
impl<T: Scalar, P> TextureFilterMode<T, P> for Nearest {
  const ENUM: FilterMode = FilterMode::Nearest;
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
  const ENUM: FilterMode = FilterMode::Linear;
  fn interpolate(t: T, one: P, other: P) -> P {
    one.lerp(other, t)
  }
}
