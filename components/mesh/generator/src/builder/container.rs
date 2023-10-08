use rendiation_mesh_core::{DynIndexContainer, GroupedMesh, IndexedMesh};

use crate::*;

impl<T> VertexBuildingContainer for Vec<T> {
  type Vertex = T;

  fn push_vertex(&mut self, v: Self::Vertex) {
    self.push(v)
  }

  fn reserve(&mut self, additional: usize) {
    self.reserve(additional)
  }
}

impl IndexedBuildingContainer for DynIndexContainer {
  fn push_index(&mut self, index: usize) {
    self.push_index_clamped_u32(index)
  }

  fn reserve(&mut self, additional: usize) {
    match self {
      DynIndexContainer::Uint16(inner) => inner.reserve(additional),
      DynIndexContainer::Uint32(inner) => inner.reserve(additional),
    }
  }
}

impl<T, U: VertexBuildingContainer> VertexBuildingContainer
  for GroupedMesh<IndexedMesh<T, U, DynIndexContainer>>
{
  type Vertex = U::Vertex;

  fn push_vertex(&mut self, v: Self::Vertex) {
    self.mesh.vertex.push_vertex(v)
  }

  fn reserve(&mut self, additional: usize) {
    self.mesh.vertex.reserve(additional)
  }
}

impl<T, U> IndexedBuildingContainer for GroupedMesh<IndexedMesh<T, U, DynIndexContainer>> {
  fn push_index(&mut self, index: usize) {
    self.mesh.index.push_index(index)
  }

  fn reserve(&mut self, additional: usize) {
    self.mesh.index.reserve(additional)
  }
}

impl<M> GroupBuildingContainer for GroupedMesh<M> {
  fn push_consequent(&mut self, count: usize) {
    self.groups.push_consequent(count)
  }
  fn extend_last(&mut self, count: usize) {
    self.groups.extend_last(count)
  }
}
