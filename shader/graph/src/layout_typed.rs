use crate::*;

/// Trait implemented for all `std140` primitives. Generally should not be
/// implemented outside this crate.
///
/// # Safety
///
///  should only be impl for std140 layout type
pub unsafe trait Std140: Copy + Zeroable + Pod + 'static {
  /// The required alignment of the type. Must be a power of two.
  ///
  /// This is distinct from the value returned by `std::mem::align_of` because
  /// `AsStd140` structs do not use Rust's alignment. This enables them to
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
    cast_slice::<Self, u8>(core::slice::from_ref(self))
  }
}

unsafe impl Std140 for f32 {
  const ALIGNMENT: usize = 4;
}

unsafe impl Std140 for f64 {
  const ALIGNMENT: usize = 8;
}

unsafe impl Std140 for i32 {
  const ALIGNMENT: usize = 4;
}

unsafe impl Std140 for u32 {
  const ALIGNMENT: usize = 4;
}

unsafe impl Std140 for Bool {
  const ALIGNMENT: usize = 4;
}

unsafe impl Std140 for Vec2<f32> {
  const ALIGNMENT: usize = 8;
}

unsafe impl Std140 for Vec3<f32> {
  const ALIGNMENT: usize = 16;
}

unsafe impl Std140 for Vec4<f32> {
  const ALIGNMENT: usize = 16;
}

unsafe impl Std140 for Vec2<u32> {
  const ALIGNMENT: usize = 8;
}

unsafe impl Std140 for Vec3<u32> {
  const ALIGNMENT: usize = 16;
}

unsafe impl Std140 for Vec4<u32> {
  const ALIGNMENT: usize = 16;
}

unsafe impl Std140 for Shader16PaddedMat2 {
  const ALIGNMENT: usize = 16;
}

unsafe impl Std140 for Shader16PaddedMat3 {
  const ALIGNMENT: usize = 16;
}

unsafe impl Std140 for Mat4<f32> {
  const ALIGNMENT: usize = 16;
}

/// Gives the number of bytes needed to make `offset` be aligned to `alignment`.
pub const fn align_offset(offset: usize, alignment: usize) -> usize {
  if alignment == 0 || offset % alignment == 0 {
    0
  } else {
    alignment - offset % alignment
  }
}

/// Max of two `usize`. Implemented because the `max` method from `Ord` cannot
/// be used in const fns.
pub const fn max(a: usize, b: usize) -> usize {
  if a > b {
    a
  } else {
    b
  }
}

/// Max of an array of `usize`. This function's implementation is funky because
/// we have no for loops!
pub const fn max_arr<const N: usize>(input: [usize; N]) -> usize {
  let mut max = 0;
  let mut i = 0;

  while i < N {
    if input[i] > max {
      max = input[i];
    }

    i += 1;
  }

  max
}

#[repr(C, align(16))]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Shader140ArrayWrapper<T> {
  pub inner: T,
}

impl<T> From<T> for Shader140ArrayWrapper<T> {
  fn from(inner: T) -> Self {
    Self { inner }
  }
}

unsafe impl<T: Zeroable> Zeroable for Shader140ArrayWrapper<T> {}
unsafe impl<T: Pod> Pod for Shader140ArrayWrapper<T> {}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shader140Array<T, const U: usize> {
  pub inner: [Shader140ArrayWrapper<T>; U],
}

impl<T, const U: usize> From<[T; U]> for Shader140Array<T, U> {
  fn from(value: [T; U]) -> Self {
    Self {
      inner: value.map(Into::into),
    }
  }
}

impl<T, const U: usize> TryFrom<Vec<T>> for Shader140Array<T, U> {
  type Error = &'static str; // todo improve

  fn try_from(value: Vec<T>) -> Result<Self, Self::Error> {
    let inner = value
      .into_iter()
      .map(Into::into)
      .collect::<Vec<_>>()
      .try_into()
      .map_err(|_| "length too big for array")?;

    Ok(Self { inner })
  }
}

/// note: rust std does't impl Default
/// https://rust-lang.github.io/project-const-generics/vision/status_quo/array_default.html
impl<T: Default + Copy, const U: usize> Default for Shader140Array<T, U> {
  fn default() -> Self {
    Self {
      inner: [Default::default(); U],
    }
  }
}

unsafe impl<T: Zeroable, const U: usize> Zeroable for Shader140Array<T, U> {}
unsafe impl<T: Pod, const U: usize> Pod for Shader140Array<T, U> {}

unsafe impl<T: Std140, const U: usize> Std140 for Shader140Array<T, U> {
  const ALIGNMENT: usize = max(16, T::ALIGNMENT);
}

/// Trait implemented for all `std430` primitives. Generally should not be
/// implemented outside this crate.
///
/// # Safety
///
///  should only be impl for std430 layout type
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
