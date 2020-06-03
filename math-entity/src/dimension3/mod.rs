pub mod box3;
pub mod sphere;
pub mod plane;
pub mod frustum;
pub mod transformation;
pub mod spherical;
pub mod ray3;
pub mod intersection;
pub mod face3;
pub mod line3;
pub mod point;

pub use box3::*;
pub use sphere::*;
pub use plane::*;
pub use frustum::*;
pub use transformation::*;
pub use spherical::*;
pub use ray3::*;
pub use intersection::*;
pub use face3::*;
pub use line3::*;
pub use point::*;

#[derive(Debug, Copy, Clone)]
pub enum Axis3 {
  X,
  Y,
  Z,
}