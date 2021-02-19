pub mod orbit;
pub use orbit::*;
pub mod fps;
pub use fps::*;

use rendiation_algebra::Mat4;

pub trait Controller<T> {
  fn update(&mut self, target: &mut T) -> bool;
}

pub trait Transformed3DControllee {
  fn matrix(&self) -> &Mat4<f32>;
  fn matrix_mut(&mut self) -> &mut Mat4<f32>;
}
