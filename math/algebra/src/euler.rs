#[repr(C)]
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq)]
pub struct Euler<T> {
  x: T,
  y: T,
  z: T,
  order: EulerOrder,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
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
