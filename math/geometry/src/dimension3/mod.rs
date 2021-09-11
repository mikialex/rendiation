pub mod bounding_impl;
pub mod box3;
pub mod contain_impl;
pub mod frustum;
pub mod intersection;
pub mod line_segment;
pub mod plane;
pub mod ray3;
pub mod sphere;
pub mod spherical;
pub mod triangle;

pub use box3::*;
pub use contain_impl::*;
pub use frustum::*;
pub use intersection::*;
pub use line_segment::*;
pub use plane::*;
pub use ray3::*;
pub use sphere::*;
pub use spherical::*;
pub use triangle::*;

use crate::{Positioned, SpaceAxis};

#[derive(Debug, Copy, Clone)]
pub enum Axis3 {
  X,
  Y,
  Z,
}

impl SpaceAxis<3> for Axis3 {}

use rendiation_algebra::Vec3;
impl<T> Positioned for Vec3<T> {
  type Position = Self;

  fn position(&self) -> &Self::Position {
    self
  }

  fn mut_position(&mut self) -> &mut Self::Position {
    self
  }
}
