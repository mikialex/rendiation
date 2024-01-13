mod bounding_impl;
mod box3;
mod frustum;
mod intersection;
mod line_segment;
mod plane;
mod ray3;
mod sphere;
mod spherical;
mod triangle;

pub use box3::*;
pub use frustum::*;
pub use line_segment::*;
pub use plane::*;
pub use ray3::*;
pub use sphere::*;
pub use spherical::*;
pub use triangle::*;

use crate::*;

#[derive(Debug, Copy, Clone)]
pub enum Axis3 {
  X,
  Y,
  Z,
}

impl SpaceAxis<3> for Axis3 {}

use rendiation_algebra::Vec3;
impl<T: Copy> Positioned for Vec3<T> {
  type Position = Self;

  fn position(&self) -> Self::Position {
    *self
  }

  fn mut_position(&mut self) -> &mut Self::Position {
    self
  }
}
