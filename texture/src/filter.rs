use rendiation_math::Scalar;

/// Texel mixing mode when sampling between texels.
pub trait TextureFilterMode {
  fn interpolate<P, T: Scalar>(&self, t: T, one: P, other: P) -> P;
}

pub struct Nearest;
impl TextureFilterMode for Nearest {
  fn interpolate<P, T: Scalar>(&self, t: T, one: P, other: P) -> P {
    todo!()
  }
}

pub struct Linear;
impl TextureFilterMode for Linear {
  fn interpolate<P, T: Scalar>(&self, t: T, one: P, other: P) -> P {
    todo!()
  }
}
