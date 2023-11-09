mod circle;
mod ellipse;
mod rectangle;
pub use circle::*;
pub use ellipse::*;
pub use rectangle::*;

use crate::*;

#[derive(Debug, Copy, Clone)]
pub enum Axis2 {
  X,
  Y,
}

impl SpaceAxis<2> for Axis2 {}

use rendiation_algebra::Vec2;
impl<T: Copy> Positioned for Vec2<T> {
  type Position = Self;

  fn position(&self) -> Self::Position {
    *self
  }

  fn mut_position(&mut self) -> &mut Self::Position {
    self
  }
}
