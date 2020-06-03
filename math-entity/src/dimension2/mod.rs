pub mod box2;
pub mod circle;
pub use box2::*;
pub use circle::*;

#[derive(Debug, Copy, Clone)]
pub enum Axis2 {
  X,
  Y,
}