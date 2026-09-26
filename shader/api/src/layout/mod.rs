pub use typed::*;

use crate::*;
mod typed;

/// Trait implemented for all `std140` primitives. Generally should not be
/// implemented outside this crate.
///
/// # Safety
///
///  should only be impl for std140 layout type
pub unsafe trait Std140: Send + Sync + Copy + Zeroable + Pod + 'static {
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
    const { assert_layout_alignment(Self::ALIGNMENT) };
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

// Mat2<f32> (and other matCx2) is intentionally not supported in std140. In WGSL uniform, the
// column stride of matCx2 is 8 bytes, but the naga glsl backend writes it into a std140 uniform
// block directly, where the column stride is 16 bytes. No host layout is correct for all backends.
//
// The matCx3 (except the Shader16PaddedMat3) is not supported in both std140 and std430, the
// column stride is 12 bytes in rust but 16 bytes in WGSL.

unsafe impl Std140 for Shader16PaddedMat3 {
  const ALIGNMENT: usize = 16;
}

unsafe impl Std140 for Mat4<f32> {
  const ALIGNMENT: usize = 16;
}

unsafe impl Std140 for Mat2x4<f32> {
  const ALIGNMENT: usize = 16;
}

unsafe impl Std140 for Mat3x4<f32> {
  const ALIGNMENT: usize = 16;
}

/// Referencing the ALIGNMENT in a monomorphized fn body forces it to be evaluated, the layout
/// checks are embedded in ALIGNMENT's const expr.
pub const fn assert_layout_alignment(alignment: usize) {
  assert!(
    alignment.is_power_of_two(),
    "layout alignment must be a power of two"
  );
}

/// Gives the number of bytes needed to make `offset` be aligned to `alignment`.
pub const fn align_offset(offset: usize, alignment: usize) -> usize {
  if alignment == 0 || offset.is_multiple_of(alignment) {
    0
  } else {
    alignment - offset % alignment
  }
}

/// Max of two `usize`. Implemented because the `max` method from `Ord` cannot
/// be used in const fns.
pub const fn max(a: usize, b: usize) -> usize {
  if a > b { a } else { b }
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

impl<T, const U: usize> Shader140Array<T, U> {
  pub fn set(&mut self, index: usize, value: T) {
    self.inner[index].inner = value;
  }
}

impl<T: Clone + Default, const U: usize> Shader140Array<T, U> {
  pub fn iter(&self) -> impl Iterator<Item = &T> + '_ {
    self.inner.iter().map(|x| &x.inner)
  }

  pub fn from_slice_clamp_or_default(slice: &[T]) -> Self {
    Self {
      inner: std::array::from_fn(|i| slice.get(i).cloned().unwrap_or_default().into()),
    }
  }
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
  const ALIGNMENT: usize = {
    // The WGSL array stride is roundUp(AlignOf(T), SizeOf(T)), and the uniform address space
    // requires it to be a multiple of 16. WGSL never pads the element for us, so array<f32, N>
    // is simply invalid in uniform, rather than having a 16 bytes stride like GLSL std140.
    assert!(
      round_up(T::ALIGNMENT, size_of::<T>()).is_multiple_of(16),
      "the element of Shader140Array must have a stride that is a multiple of 16 bytes \
       (for example use Vec4 instead of f32/Vec2)"
    );
    max(16, T::ALIGNMENT)
  };
}

/// Trait implemented for all `std430` primitives. Generally should not be
/// implemented outside this crate.
///
/// # Safety
///
///  should only be impl for std430 layout type
pub unsafe trait Std430: Send + Sync + Copy + Zeroable + Pod {
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
    const { assert_layout_alignment(Self::ALIGNMENT) };
    bytes_of(self)
  }

  /// we not require this method on std140 because we never need to read back from uniform buffer
  fn from_bytes(bytes: &[u8]) -> Self {
    const { assert_layout_alignment(Self::ALIGNMENT) };
    // should we do copy unaligned?
    *bytemuck::from_bytes(bytes)
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

unsafe impl Std430 for Mat3x2<f32> {
  const ALIGNMENT: usize = 8;
}

unsafe impl Std430 for Mat4x2<f32> {
  const ALIGNMENT: usize = 8;
}

unsafe impl Std430 for Mat2x4<f32> {
  const ALIGNMENT: usize = 16;
}

unsafe impl Std430 for Mat3x4<f32> {
  const ALIGNMENT: usize = 16;
}

unsafe impl<T: Std430, const U: usize> Std430 for [T; U]
where
  Self: Pod,
{
  const ALIGNMENT: usize = {
    assert_std430_array_element::<T>();
    T::ALIGNMENT
  };
}

/// The rust array has no padding between elements, so the rust stride is SizeOf(T) but the
/// WGSL stride is roundUp(AlignOf(T), SizeOf(T)), for example, the vec3 in rust is 12 bytes
/// but the array<vec3> stride is 16 bytes in WGSL.
const fn assert_std430_array_element<T: Std430>() {
  assert!(
    size_of::<T>().is_multiple_of(T::ALIGNMENT),
    "the size of the array element must be a multiple of its alignment, \
     so the rust array stride can match the WGSL one (for example use Vec4 instead of Vec3)"
  );
}

/// # Safety
/// should only be impl on std430 layout type
pub unsafe trait Std430MaybeUnsized: Send + Sync {
  fn bytes(&self) -> &[u8];
  fn from_bytes_into_boxed(bytes: &[u8]) -> Box<Self>;
}

unsafe impl<T: Std430 + Send + Sync> Std430MaybeUnsized for T {
  fn bytes(&self) -> &[u8] {
    self.as_bytes()
  }
  fn from_bytes_into_boxed(bytes: &[u8]) -> Box<Self> {
    Box::new(Self::from_bytes(bytes))
  }
}
unsafe impl<T: Std430 + Send + Sync> Std430MaybeUnsized for [T] {
  fn bytes(&self) -> &[u8] {
    const { assert_std430_array_element::<T>() };
    bytemuck::cast_slice(self)
  }
  fn from_bytes_into_boxed(bytes: &[u8]) -> Box<Self> {
    const { assert_std430_array_element::<T>() };
    from_bytes_into_boxed_slice(bytes)
  }
}

pub fn from_bytes_into_boxed_slice<T: Pod>(bytes: &[u8]) -> Box<[T]> {
  let slice: &[T] = bytemuck::cast_slice(bytes);
  // we should try unsafe here, todo
  // https://www.reddit.com/r/rust/comments/jzwwqb/about_creating_a_boxed_slice/
  Vec::from_iter(slice.iter().copied()).into_boxed_slice()
}
