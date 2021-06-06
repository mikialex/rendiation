use std::marker::PhantomData;

use crate::{
  mesh::IndexPrimitiveTopologyMeta, mesh::IndexedPrimitiveData, mesh::PrimitiveTopologyMeta,
  mesh::TriangleList, vertex::Vertex,
};

use super::{AnyIndexMesh, AnyMesh, MeshDataContainer};

pub struct IndexedMeshView<'a, I, V = Vertex, T = TriangleList, U = Vec<V>> {
  pub data: &'a U,
  pub index: &'a Vec<I>,
  _v_phantom: PhantomData<V>,
  _phantom: PhantomData<T>,
}

#[allow(clippy::ptr_arg)]
impl<'a, I, V, T, U> IndexedMeshView<'a, I, V, T, U> {
  pub fn new(v: &'a U, index: &'a Vec<I>) -> Self {
    Self {
      data: v,
      index,
      _v_phantom: PhantomData,
      _phantom: PhantomData,
    }
  }
}

impl<'a, I, V, T, U> AnyMesh for IndexedMeshView<'a, I, V, T, U>
where
  V: Copy,
  T: IndexPrimitiveTopologyMeta<I, V>,
  <T as PrimitiveTopologyMeta<V>>::Primitive: IndexedPrimitiveData<I, V, U, Vec<I>>,
  U: MeshDataContainer<V>,
{
  type Primitive = T::Primitive;

  #[inline(always)]
  fn draw_count(&self) -> usize {
    self.index.len()
  }

  #[inline(always)]
  fn primitive_count(&self) -> usize {
    (self.index.len() - T::STRIDE) / T::STEP + 1
  }

  #[inline(always)]
  fn primitive_at(&self, primitive_index: usize) -> Self::Primitive {
    let index = primitive_index * T::STEP;
    T::Primitive::from_indexed_data(&self.index, &self.data, index)
  }
}

impl<'a, I, V, T, U> AnyIndexMesh for IndexedMeshView<'a, I, V, T, U>
where
  V: Copy,
  T: IndexPrimitiveTopologyMeta<I, V>,
  T::Primitive: IndexedPrimitiveData<I, V, U, Vec<I>>,
  U: MeshDataContainer<V>,
{
  type IndexPrimitive = <T::Primitive as IndexedPrimitiveData<I, V, U, Vec<I>>>::IndexIndicator;

  fn index_primitive_at(&self, primitive_index: usize) -> Self::IndexPrimitive {
    let index = primitive_index * T::STEP;
    T::Primitive::create_index_indicator(&self.index, index)
  }
}
