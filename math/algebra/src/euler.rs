use crate::*;

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq, Facet)]
pub struct Euler<T> {
  x: T,
  y: T,
  z: T,
  order: EulerOrder,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Facet)]
pub enum EulerOrder {
  XYZ,
  YXZ,
  ZXY,
  ZYX,
  YZX,
  XZY,
}

impl Default for EulerOrder {
  fn default() -> Self {
    Self::XYZ
  }
}
