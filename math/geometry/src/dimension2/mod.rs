pub mod circle;
pub mod rectangle;
pub use circle::*;
pub use rectangle::*;

use crate::SpaceAxis;

#[derive(Debug, Copy, Clone)]
pub enum Axis2 {
  X,
  Y,
}

impl SpaceAxis<2> for Axis2 {}
