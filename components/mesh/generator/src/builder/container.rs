use rendiation_mesh_core::DynIndexContainer;

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
