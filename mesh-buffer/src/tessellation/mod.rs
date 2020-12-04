use crate::{
  geometry::{IndexType, IndexedGeometry, NoneIndexedGeometry, TriangleList},
  vertex::Vertex,
};

pub mod plane;
pub use plane::*;
pub mod sphere;
pub use sphere::*;

// todo add support for index overflow check
pub trait IndexedTessellator<T = Vertex, I: IndexType = u16, P = TriangleList> {
  fn create_mesh(&self) -> IndexedGeometry<I, T, P>;
}

pub trait BufferTessellator<T = Vertex, P = TriangleList> {
  fn create_mesh(&self) -> NoneIndexedGeometry<T, P>;
}
