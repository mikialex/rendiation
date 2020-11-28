use crate::*;
use std::mem;

impl AsRef<[u8]> for Mat4<f32> {
  #[inline]
  fn as_ref(&self) -> &[u8] {
    unsafe { mem::transmute::<&Mat4<f32>, &[u8; 16 * 4]>(self) }
  }
}

impl AsRef<[u8]> for Vec3<f32> {
  #[inline]
  fn as_ref(&self) -> &[u8] {
    unsafe { mem::transmute::<&Vec3<f32>, &[u8; 3 * 4]>(self) }
  }
}
