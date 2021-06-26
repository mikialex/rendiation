use super::{
  super::{PrimitiveTopologyMeta, TriangleList},
  AbstractIndexMesh, AbstractMesh,
};
use crate::{mesh::IndexedPrimitiveData, vertex::Vertex};
use core::marker::PhantomData;
use std::{
  convert::{TryFrom, TryInto},
  fmt::Debug,
  hash::Hash,
};

pub trait IndexType:
  TryFrom<usize, Error: Debug> + TryInto<usize, Error: Debug> + Copy + Eq + Ord + Hash
{
}
impl IndexType for u32 {}
impl IndexType for u16 {}

/// A indexed mesh that use vertex as primitive;
pub struct IndexedMesh<I = u16, V = Vertex, T = TriangleList, U = Vec<V>, IU = Vec<I>> {
  pub data: U,
  pub index: IU,
  _i_phantom: PhantomData<I>,
  _v_phantom: PhantomData<V>,
  _phantom: PhantomData<T>,
}

impl<I, V, T, U> From<(U, Vec<I>)> for IndexedMesh<I, V, T, U> {
  fn from(item: (U, Vec<I>)) -> Self {
    IndexedMesh::new(item.0, item.1)
  }
}

impl<I, V, T, U, IU> IndexedMesh<I, V, T, U, IU> {
  pub fn new(v: U, index: IU) -> Self {
    Self {
      data: v,
      index,
      _i_phantom: PhantomData,
      _v_phantom: PhantomData,
      _phantom: PhantomData,
    }
  }
}

impl<I, V, T, U, IU> AbstractMesh for IndexedMesh<I, V, T, U, IU>
where
  V: Copy,
  U: AsRef<[V]>,
  IU: AsRef<[I]>,
  T: PrimitiveTopologyMeta<V>,
  <T as PrimitiveTopologyMeta<V>>::Primitive: IndexedPrimitiveData<I, V, U, IU>,
{
  type Primitive = T::Primitive;

  #[inline(always)]
  fn draw_count(&self) -> usize {
    self.index.as_ref().len()
  }

  #[inline(always)]
  fn primitive_count(&self) -> usize {
    (self.index.as_ref().len() - T::STRIDE) / T::STEP + 1
  }

  #[inline(always)]
  fn primitive_at(&self, primitive_index: usize) -> Self::Primitive {
    let index = primitive_index * T::STEP;
    T::Primitive::from_indexed_data(&self.index, &self.data, index)
  }
}

impl<I, V, T, U, IU> AbstractIndexMesh for IndexedMesh<I, V, T, U, IU>
where
  V: Copy,
  U: AsRef<[V]>,
  IU: AsRef<[I]>,
  T: PrimitiveTopologyMeta<V>,
  T::Primitive: IndexedPrimitiveData<I, V, U, IU>,
{
  type IndexPrimitive = <T::Primitive as IndexedPrimitiveData<I, V, U, IU>>::IndexIndicator;

  #[inline(always)]
  fn index_primitive_at(&self, primitive_index: usize) -> Self::IndexPrimitive {
    let index = primitive_index * T::STEP;
    T::Primitive::create_index_indicator(&self.index, index)
  }
}
