use crate::{Mat4, NDCSpaceMapper};

pub mod perspective;
pub use perspective::*;
pub mod orth;
pub use orth::*;

pub trait Projection: Send + Sync {
  fn update_projection<S: NDCSpaceMapper>(&self, projection: &mut Mat4<f32>);
}

pub trait ResizableProjection: Projection {
  fn resize(&mut self, size: (f32, f32));
}
