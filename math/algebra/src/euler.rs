use crate::*;

#[repr(C)]
#[derive(Serialize, Deserialize)]
#[derive(Debug, Copy, Clone, Default, Hash, Eq, PartialEq, Facet)]
pub struct Euler<T> {
  x: T,
  y: T,
  z: T,
  order: EulerOrder,
}

#[repr(C)]
#[derive(Serialize, Deserialize)]
#[derive(Default, Debug, Copy, Clone, Hash, Eq, PartialEq, Facet)]
pub enum EulerOrder {
  #[default]
  XYZ,
  YXZ,
  ZXY,
  ZYX,
  YZX,
  XZY,
}
