use crate::Mat4;

pub mod perspective;
pub use perspective::*;
pub mod orth;
pub use orth::*;

pub trait Projection {
  fn update_projection(&self, projection: &mut Mat4<f32>);
}

pub trait ResizableProjection: Projection {
  fn resize(&mut self, size: (f32, f32));
}
