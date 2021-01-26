use rendiation_math::{Scalar, Vec2};
use rendiation_ral::AddressMode;

/// How edges should be handled in texture addressing.
pub trait TextureAddressMode {
  fn to_ral_enum(&self) -> AddressMode;
  /// correct uv to [1, 1]
  fn correct<T: Scalar>(&self, uv: Vec2<T>) -> Vec2<T>;
}

/// Clamp the value to the edge of the texture
///
/// -0.25 -> 0.0
/// 1.25  -> 1.0
pub struct ClampToEdge;
impl TextureAddressMode for ClampToEdge {
  fn to_ral_enum(&self) -> AddressMode {
    AddressMode::ClampToEdge
  }
  fn correct<T: Scalar>(&self, uv: Vec2<T>) -> Vec2<T> {
    uv.map(|c| c.max(T::zero()).min(T::one()))
  }
}

/// Repeat the texture in a tiling fashion
///
/// -0.25 -> 0.75
/// 1.25 -> 0.25
pub struct Repeat;
impl TextureAddressMode for Repeat {
  fn to_ral_enum(&self) -> AddressMode {
    AddressMode::Repeat
  }
  fn correct<T: Scalar>(&self, uv: Vec2<T>) -> Vec2<T> {
    todo!()
    // uv.map(|c| c % T::one())
  }
}

/// Repeat the texture, mirroring it every repeat
///
/// -0.25 -> 0.25
/// 1.25 -> 0.75
pub struct MirrorRepeat;
impl TextureAddressMode for MirrorRepeat {
  fn to_ral_enum(&self) -> AddressMode {
    AddressMode::MirrorRepeat
  }
  fn correct<T: Scalar>(&self, uv: Vec2<T>) -> Vec2<T> {
    todo!()
  }
}
