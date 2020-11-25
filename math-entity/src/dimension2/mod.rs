pub mod circle;
pub mod rectangle;
pub use circle::*;
pub use rectangle::*;
use rendiation_math::Vec2;

#[derive(Debug, Copy, Clone)]
pub enum Axis2 {
  X,
  Y,
}
