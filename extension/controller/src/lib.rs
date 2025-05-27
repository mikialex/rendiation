#![allow(clippy::collapsible_match)]
#![allow(clippy::single_match)]

mod orbit;
pub use orbit::*;
mod fps;
pub use fps::*;
use rendiation_algebra::*;

#[derive(Clone, Copy)]
pub struct InputBound {
  pub origin: Vec2<f32>,
  pub size: Vec2<f32>,
}

impl InputBound {
  pub fn is_point_in(&self, point: Vec2<f32>) -> bool {
    point.x >= self.origin.x
      && point.y >= self.origin.y
      && point.x <= self.origin.x + self.size.x
      && point.y <= self.origin.y + self.size.y
  }
}
