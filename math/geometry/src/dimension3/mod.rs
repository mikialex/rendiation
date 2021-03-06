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

use crate::SpaceAxis;

#[derive(Debug, Copy, Clone)]
pub enum Axis3 {
  X,
  Y,
  Z,
}

impl SpaceAxis<3> for Axis3 {}
