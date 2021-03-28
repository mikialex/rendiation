use rendiation_algebra::Scalar;

/// How edges should be handled in texture addressing.
#[repr(C)]
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum AddressMode {
  /// Clamp the value to the edge of the texture
  ///
  /// -0.25 -> 0.0
  /// 1.25  -> 1.0
  ClampToEdge = 0,
  /// Repeat the texture in a tiling fashion
  ///
  /// -0.25 -> 0.75
  /// 1.25 -> 0.25
  Repeat = 1,
  /// Repeat the texture, mirroring it every repeat
  ///
  /// -0.25 -> 0.25
  /// 1.25 -> 0.75
  MirrorRepeat = 2,
  /// Clamp the value to the border of the texture
  /// Requires feature [`Features::ADDRESS_MODE_CLAMP_TO_BORDER`]
  ///
  /// -0.25 -> border
  /// 1.25 -> border
  ClampToBorder = 3,
}

/// How edges should be handled in texture addressing.
pub trait TextureAddressMode {
  const ENUM: AddressMode;
  /// correct uv to [1, 1]
  fn correct<T: Scalar>(uv: T) -> T;
}

/// Clamp the value to the edge of the texture
///
/// -0.25 -> 0.0
/// 1.25  -> 1.0
pub struct ClampToEdge;
impl TextureAddressMode for ClampToEdge {
  const ENUM: AddressMode = AddressMode::ClampToEdge;
  fn correct<T: Scalar>(uv: T) -> T {
    uv.max(T::zero()).min(T::one())
  }
}

#[test]
fn clamp() {
  assert_eq!(ClampToEdge::correct(-0.25), 0.0);
  assert_eq!(ClampToEdge::correct(1.25), 1.0);
}

/// Repeat the texture in a tiling fashion
///
/// -0.25 -> 0.75
/// 1.25 -> 0.25
pub struct Repeat;
impl TextureAddressMode for Repeat {
  const ENUM: AddressMode = AddressMode::Repeat;
  fn correct<T: Scalar>(uv: T) -> T {
    uv % T::one()
  }
}

#[test]
fn repeat() {
  assert_eq!(Repeat::correct(-0.25), 0.75);
  assert_eq!(Repeat::correct(1.25), 0.25);
}

/// Repeat the texture, mirroring it every repeat
///
/// -0.25 -> 0.25
/// 1.25 -> 0.75
pub struct MirrorRepeat;
impl TextureAddressMode for MirrorRepeat {
  const ENUM: AddressMode = AddressMode::MirrorRepeat;
  fn correct<T: Scalar>(_uv: T) -> T {
    todo!()
  }
}

#[test]
fn mirror_repeat() {
  assert_eq!(MirrorRepeat::correct(-0.25), 0.25);
  assert_eq!(MirrorRepeat::correct(1.25), 0.75);
}
