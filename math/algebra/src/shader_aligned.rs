use bytemuck::*;

use crate::*;

/// A host shareable(32 bits) `bool` type used in shader code. aka "Big Bool"
#[derive(Clone, Copy, Eq, PartialEq, Zeroable, Pod, Default, Hash)]
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

use core::fmt::{Debug, Formatter};

impl Debug for Bool {
  fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
    write!(f, "Bool({:?})", bool::from(*self))
  }
}

#[repr(C)]
#[rustfmt::skip]
#[derive(Clone, Copy, Zeroable, Pod, PartialEq, Default, Debug)]
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

#[repr(C)]
#[rustfmt::skip]
#[derive(Clone, Copy, Zeroable, Pod, PartialEq, Default, Debug)]
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
