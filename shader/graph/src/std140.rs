use crate::*;

pub trait Std140TypeMapper {
  type StorageType: Std140;
}

impl<T: Std140> Std140TypeMapper for T {
  default type StorageType = Self;
}

/// Trait implemented for all `std140` primitives. Generally should not be
/// implemented outside this crate.
pub unsafe trait Std140: Copy + Zeroable + Pod {
  /// The required alignment of the type. Must be a power of two.
  ///
  /// This is distinct from the value returned by `std::mem::align_of` because
  /// `AsStd140` structs do not use Rust's alignment. This enables them to
  /// control and zero their padding bytes, making converting them to and from
  /// slices safe.
  const ALIGNMENT: usize;

  /// Whether this type requires a padding at the end (ie, is a struct or an array
  /// of primitives).
  /// See <https://www.khronos.org/registry/OpenGL/specs/gl/glspec45.core.pdf#page=159>
  /// (rule 4 and 9)
  const PAD_AT_END: bool = false;

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
impl Std140TypeMapper for bool {
  type StorageType = Bool;
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

unsafe impl Std140 for Shader140Mat2 {
  const ALIGNMENT: usize = 16;
  const PAD_AT_END: bool = true;
}
impl Std140TypeMapper for Mat2<f32> {
  type StorageType = Shader140Mat2;
}

#[repr(C)]
#[rustfmt::skip]
#[derive(Clone, Copy, Zeroable, Pod)]
pub struct Shader140Mat2{
	pub a1:f32, pub a2:f32, _pad1: [f32; 2],
	pub b1:f32, pub b2:f32, _pad2: [f32; 2],
}

unsafe impl Std140 for Shader140Mat3 {
  const ALIGNMENT: usize = 16;
  const PAD_AT_END: bool = true;
}
impl Std140TypeMapper for Mat3<f32> {
  type StorageType = Shader140Mat3;
}

#[repr(C)]
#[rustfmt::skip]
#[derive(Clone, Copy, Zeroable, Pod)]
pub struct Shader140Mat3 {
  pub a1: f32, pub a2: f32, pub a3: f32, _pad1: f32,
  pub b1: f32, pub b2: f32, pub b3: f32, _pad2: f32,
  pub c1: f32, pub c2: f32, pub c3: f32, _pad3: f32,
}

unsafe impl Std140 for Mat4<f32> {
  const ALIGNMENT: usize = 16;
  const PAD_AT_END: bool = true;
}

/// GLSL's `bool` type.
///
/// Boolean values in GLSL are 32 bits, in contrast with Rust's 8 bit bools.
#[derive(Clone, Copy, Eq, PartialEq, Zeroable, Pod)]
#[repr(transparent)]
pub struct Bool(u32);

impl From<bool> for Bool {
  fn from(v: bool) -> Self {
    Self(v as u32)
  }
}

impl From<Bool> for bool {
  fn from(v: Bool) -> Self {
    v.0 != 0
  }
}

use core::fmt::{Debug, Formatter};
impl Debug for Bool {
  fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
    write!(f, "Bool({:?})", bool::from(*self))
  }
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
