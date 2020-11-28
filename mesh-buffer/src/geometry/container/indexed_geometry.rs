use super::{
  super::{PrimitiveTopology, TriangleList},
  AnyGeometry, AnyIndexGeometry, GeometryDataContainer,
};
use crate::{
  geometry::{IndexPrimitiveTopology, IndexedPrimitiveData},
  vertex::Vertex,
};
use core::marker::PhantomData;
use rendiation_math_entity::Positioned;
use std::hash::Hash;

pub trait IntoUsize {
  fn into_usize(&self) -> usize;
  fn from_usize(v: usize) -> Self;
}
pub trait IndexType: IntoUsize + Copy + Eq + Ord + Hash {}

impl IndexType for u16 {}
impl IntoUsize for u16 {
  #[inline(always)]
  fn into_usize(&self) -> usize {
    *self as usize
  }
  #[inline(always)]
  fn from_usize(v: usize) -> Self {
    v as Self
  }
}

impl IndexType for u32 {}
impl IntoUsize for u32 {
  #[inline(always)]
  fn into_usize(&self) -> usize {
    *self as usize
  }
  #[inline(always)]
  fn from_usize(v: usize) -> Self {
    v as Self
  }
}

/// A indexed geometry that use vertex as primitive;
pub struct IndexedGeometry<I = u16, V = Vertex, T = TriangleList, U = Vec<V>> {
  pub data: U,
  pub index: Vec<I>,
  _v_phantom: PhantomData<V>,
  _phantom: PhantomData<T>,
}

impl<I, V, T, U> From<(U, Vec<I>)> for IndexedGeometry<I, V, T, U> {
  fn from(item: (U, Vec<I>)) -> Self {
    IndexedGeometry::new(item.0, item.1)
  }
}

impl<V, I, T, U> IndexedGeometry<I, V, T, U> {
  pub fn new(v: U, index: Vec<I>) -> Self {
    Self {
      data: v,
      index,
      _v_phantom: PhantomData,
      _phantom: PhantomData,
    }
  }
}

impl<I, V, T, U> AnyGeometry for IndexedGeometry<I, V, T, U>
where
  V: Positioned<f32, 3>,
  T: IndexPrimitiveTopology<I, V>,
  <T as PrimitiveTopology<V>>::Primitive: IndexedPrimitiveData<I, V>,
  U: GeometryDataContainer<V>,
{
  type Primitive = T::Primitive;

  #[inline(always)]
  fn primitive_count(&self) -> usize {
    (self.index.len() - T::STRIDE) / T::STEP + 1
  }

  #[inline(always)]
  fn primitive_at(&self, primitive_index: usize) -> Self::Primitive {
    let index = primitive_index * T::STEP;
    T::Primitive::from_indexed_data(&self.index, self.data.as_ref(), index)
  }
}

impl<I, V, T, U> AnyIndexGeometry for IndexedGeometry<I, V, T, U>
where
  V: Positioned<f32, 3>,
  T: IndexPrimitiveTopology<I, V>,
  T::Primitive: IndexedPrimitiveData<I, V>,
  U: GeometryDataContainer<V>,
{
  type IndexPrimitive = <T::Primitive as IndexedPrimitiveData<I, V>>::IndexIndicator;

  #[inline(always)]
  fn index_primitive_at(&self, primitive_index: usize) -> Self::IndexPrimitive {
    let index = primitive_index * T::STEP;
    T::Primitive::create_index_indicator(&self.index, index)
  }
}
