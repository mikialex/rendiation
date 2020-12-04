use crate::{
  geometry::{IndexType, IndexedGeometry, NoneIndexedGeometry, TriangleList},
  vertex::Vertex,
};

pub mod cube;
pub mod plane;
pub mod sphere;
pub use cube::*;
pub use plane::*;
pub use sphere::*;

// todo add support for index overflow check
pub trait IndexedGeometryTessellator<T = Vertex, I: IndexType = u16, P = TriangleList> {
  fn tessellate(&self) -> IndexedGeometry<I, T, P>;
}

pub trait NoneIndexedGeometryTessellator<T = Vertex, P = TriangleList> {
  fn tessellate(&self) -> NoneIndexedGeometry<T, P>;
}
