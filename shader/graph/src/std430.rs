use crate::*;

/// Trait implemented for all `std430` primitives. Generally should not be
/// implemented outside this crate.
///
/// # Safety
///
///  should only be impl for std140 layout type
pub unsafe trait Std430: Copy + Zeroable + Pod {
  /// The required alignment of the type. Must be a power of two.
  ///
  /// This is distinct from the value returned by `std::mem::align_of` because
  /// `AsStd430` structs do not use Rust's alignment. This enables them to
  /// control and zero their padding bytes, making converting them to and from
  /// slices safe.
  const ALIGNMENT: usize;

  /// Casts the type to a byte array. Implementors should not override this
  /// method.
  ///
  /// # Safety
  /// This is always safe due to the requirements of [`bytemuck::Pod`] being a
  /// prerequisite for this trait.
  fn as_bytes(&self) -> &[u8] {
    bytes_of(self)
  }
}

unsafe impl Std430 for f32 {
  const ALIGNMENT: usize = 4;
}

unsafe impl Std430 for f64 {
  const ALIGNMENT: usize = 8;
}

unsafe impl Std430 for i32 {
  const ALIGNMENT: usize = 4;
}

unsafe impl Std430 for u32 {
  const ALIGNMENT: usize = 4;
}

unsafe impl Std430 for Bool {
  const ALIGNMENT: usize = 4;
}

unsafe impl Std430 for Vec2<f32> {
  const ALIGNMENT: usize = 8;
}

unsafe impl Std430 for Vec3<f32> {
  const ALIGNMENT: usize = 16;
}

unsafe impl Std430 for Vec4<f32> {
  const ALIGNMENT: usize = 16;
}

unsafe impl Std430 for Vec2<u32> {
  const ALIGNMENT: usize = 8;
}

unsafe impl Std430 for Vec3<u32> {
  const ALIGNMENT: usize = 16;
}

unsafe impl Std430 for Vec4<u32> {
  const ALIGNMENT: usize = 16;
}

unsafe impl Std430 for Mat2<f32> {
  const ALIGNMENT: usize = 8;
}

unsafe impl Std430 for Shader16PaddedMat3 {
  const ALIGNMENT: usize = 16;
}

unsafe impl Std430 for Mat4<f32> {
  const ALIGNMENT: usize = 16;
}

unsafe impl<T: Std430, const U: usize> Std430 for [T; U]
where
  Self: Pod,
{
  const ALIGNMENT: usize = T::ALIGNMENT;
}
