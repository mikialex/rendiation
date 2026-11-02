//! The host side types that match the shader side memory layout, for the types that have
//! different layout between rust and WGSL uniform / storage buffer.

use core::fmt::{Debug, Formatter};

use bytemuck::*;

use crate::*;

/// A host shareable(32 bits) `bool` type used in shader code. aka "Big Bool"
#[derive(Serialize, Deserialize)]
#[derive(Clone, Copy, Eq, PartialEq, Zeroable, Pod, Default, Hash, Facet)]
#[repr(transparent)]
pub struct Bool(pub u32);

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

impl Debug for Bool {
  fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
    write!(f, "Bool({:?})", bool::from(*self))
  }
}

/// The [`Mat3<f32>`] with each column padded to 16 bytes, which is the layout of WGSL `mat3x3<f32>`.
#[repr(C)]
#[rustfmt::skip]
#[derive(Serialize, Deserialize)]
#[derive(Clone, Copy, Zeroable, Pod, PartialEq, Default, Debug, Facet)]
pub struct Shader16PaddedMat3 {
  pub a1: f32, pub a2: f32, pub a3: f32, _pad1: f32,
  pub b1: f32, pub b2: f32, pub b3: f32, _pad2: f32,
  pub c1: f32, pub c2: f32, pub c3: f32, _pad3: f32,
}

impl From<Mat3<f32>> for Shader16PaddedMat3 {
  #[rustfmt::skip]
  fn from(v: Mat3<f32>) -> Self {
    Self {
      a1: v.a1, a2: v.a2, a3: v.a3,
      b1: v.b1, b2: v.b2, b3: v.b3,
      c1: v.c1, c2: v.c2, c3: v.c3,
      ..Default::default()
    }
  }
}

impl From<Shader16PaddedMat3> for Mat3<f32> {
  #[rustfmt::skip]
  fn from(v: Shader16PaddedMat3) -> Self {
    Self {
      a1: v.a1, a2: v.a2, a3: v.a3,
      b1: v.b1, b2: v.b2, b3: v.b3,
      c1: v.c1, c2: v.c2, c3: v.c3,
    }
  }
}

/// The [`Mat2<f32>`] with each column padded to 16 bytes, which is the std140 style layout. Note
/// that WGSL `mat2x2<f32>` is not padded (size 16, align 8), use [`Mat2<f32>`] directly for it.
#[repr(C)]
#[rustfmt::skip]
#[derive(Serialize, Deserialize)]
#[derive(Clone, Copy, Zeroable, Pod, PartialEq, Default, Debug, Facet)]
pub struct Shader16PaddedMat2 {
  pub a1:f32, pub a2:f32, _pad1: [f32; 2],
  pub b1:f32, pub b2:f32, _pad2: [f32; 2],
}

impl From<Mat2<f32>> for Shader16PaddedMat2 {
  #[rustfmt::skip]
  fn from(v: Mat2<f32>) -> Self {
    Self {
      a1: v.a1, a2: v.a2,
      b1: v.b1, b2: v.b2,
      ..Default::default()
    }
  }
}

impl From<Shader16PaddedMat2> for Mat2<f32> {
  #[rustfmt::skip]
  fn from(v: Shader16PaddedMat2) -> Self {
    Self {
      a1: v.a1, a2: v.a2,
      b1: v.b1, b2: v.b2,
    }
  }
}

#[test]
fn shader_aligned_conversion() {
  assert_eq!(std::mem::size_of::<Bool>(), 4);
  assert!(bool::from(Bool::from(true)));
  assert!(!bool::from(Bool::from(false)));
  assert!(bool::from(Bool(2)));
  assert_eq!(format!("{:?}", Bool::from(true)), "Bool(true)");

  assert_eq!(std::mem::size_of::<Shader16PaddedMat3>(), 48);
  let m = Mat3::new(1., 2., 3., 4., 5., 6., 7., 8., 9.);
  let padded = Shader16PaddedMat3::from(m);
  let raw: &[f32] = bytemuck::cast_slice(std::slice::from_ref(&padded));
  assert_eq!(raw, &[1., 2., 3., 0., 4., 5., 6., 0., 7., 8., 9., 0.]);
  assert_eq!(Mat3::from(padded), m);

  assert_eq!(std::mem::size_of::<Shader16PaddedMat2>(), 32);
  let m = Mat2::new(1., 2., 3., 4.);
  let padded = Shader16PaddedMat2::from(m);
  let raw: &[f32] = bytemuck::cast_slice(std::slice::from_ref(&padded));
  assert_eq!(raw, &[1., 2., 0., 0., 3., 4., 0., 0.]);
  assert_eq!(Mat2::from(padded), m);
}
