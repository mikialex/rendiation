pub mod box3;
pub mod sphere;
pub mod plane;
pub mod frustum;
pub mod spherical;
pub mod ray3;
pub mod intersection;
pub mod triangle;

pub use box3::*;
pub use sphere::*;
pub use plane::*;
pub use frustum::*;
pub use spherical::*;
pub use ray3::*;
pub use intersection::*;
pub use triangle::*;
use rendiation_math::Vec3;

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
  fn position(&self) -> Vec3<f32> {
    *self
  }
}
