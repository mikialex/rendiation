use crate::{
  group::GroupedMesh,
  mesh::{IndexType, IndexedMesh, NoneIndexedMesh, TriangleList},
  vertex::Vertex,
};

pub mod cube;
pub mod cylinder;
pub mod plane;
pub mod sphere;
pub use cube::*;
pub use cylinder::*;
pub use plane::*;
pub use sphere::*;

// todo add support for index overflow check
pub trait IndexedMeshTessellator<T = Vertex, I: IndexType = u16, P = TriangleList> {
  fn tessellate(&self) -> GroupedMesh<IndexedMesh<I, T, P, Vec<T>, Vec<u16>>>;
}

pub trait NoneIndexedMeshTessellator<T = Vertex, P = TriangleList> {
  fn tessellate(&self) -> GroupedMesh<NoneIndexedMesh<T, P, Vec<T>>>;
}
