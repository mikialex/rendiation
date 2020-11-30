use crate::{geometry::IndexType, vertex::Vertex};

pub mod plane;
pub use plane::*;
pub mod sphere;
pub use sphere::*;

// todo add support for index overflow check
pub trait IndexedBufferTessellator<T = Vertex, I: IndexType = u16> {
  type TessellationParameter;
  fn create_mesh(&self, p: &Self::TessellationParameter) -> (Vec<T>, Vec<I>);
}

pub trait BufferTessellator<T = Vertex> {
  type TessellationParameter;
  fn create_mesh(&self, p: &Self::TessellationParameter) -> Vec<T>;
}
