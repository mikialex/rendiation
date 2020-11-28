pub mod circle;
pub mod rectangle;
pub use circle::*;
pub use rectangle::*;

#[derive(Debug, Copy, Clone)]
pub enum Axis2 {
  X,
  Y,
}
