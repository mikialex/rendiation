use std::marker::PhantomData;

use crate::{mesh::PrimitiveData, mesh::PrimitiveTopologyMeta, mesh::TriangleList, vertex::Vertex};

use super::{AnyMesh, MeshDataContainer};

pub struct NoneIndexedMeshView<'a, V = Vertex, T = TriangleList, U = Vec<V>> {
  pub data: &'a U,
  _v_phantom: PhantomData<V>,
  _phantom: PhantomData<T>,
}

impl<'a, V, T, U> NoneIndexedMeshView<'a, V, T, U> {
  pub fn new(v: &'a U) -> Self {
    Self {
      data: v,
      _v_phantom: PhantomData,
      _phantom: PhantomData,
    }
  }
}

impl<'a, V, T, U> AnyMesh for NoneIndexedMeshView<'a, V, T, U>
where
  T: PrimitiveTopologyMeta<V>,
  U: MeshDataContainer<V>,
  T::Primitive: PrimitiveData<V, U>,
{
  type Primitive = T::Primitive;

  #[inline(always)]
  fn draw_count(&self) -> usize {
    self.data.as_ref().len()
  }

  #[inline(always)]
  fn primitive_count(&self) -> usize {
    (self.data.as_ref().len() - T::STRIDE) / T::STEP + 1
  }

  #[inline(always)]
  fn primitive_at(&self, primitive_index: usize) -> Self::Primitive {
    let index = primitive_index * T::STEP;
    T::Primitive::from_data(&self.data, index)
  }
}
