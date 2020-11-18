pub mod bounding_impl;
pub mod box3;
pub mod frustum;
pub mod intersection;
pub mod line_segment;
pub mod plane;
pub mod ray3;
pub mod sphere;
pub mod spherical;
pub mod triangle;

pub use box3::*;
pub use frustum::*;
pub use intersection::*;
pub use line_segment::*;
pub use plane::*;
pub use ray3::*;
use rendiation_math::Vec3;
pub use sphere::*;
pub use spherical::*;
pub use triangle::*;

#[derive(Debug, Copy, Clone)]
pub enum Axis3 {
  X,
  Y,
  Z,
}

pub trait Positioned3D: Copy {
  fn position(&self) -> Vec3<f32>;
}

impl Positioned3D for Vec3<f32> {
  #[inline(always)]
  fn position(&self) -> Vec3<f32> {
    *self
  }
}
