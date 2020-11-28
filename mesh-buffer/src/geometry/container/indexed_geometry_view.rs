use std::marker::PhantomData;

use crate::{
  geometry::IndexPrimitiveTopology, geometry::IndexedPrimitiveData, geometry::PrimitiveTopology,
  geometry::TriangleList, vertex::Vertex,
};
use rendiation_math_entity::Positioned;

use super::{AnyGeometry, AnyIndexGeometry, GeometryDataContainer};

pub struct IndexedGeometryView<'a, I, V = Vertex, T = TriangleList, U = Vec<V>> {
  pub data: &'a U,
  pub index: &'a Vec<I>,
  _v_phantom: PhantomData<V>,
  _phantom: PhantomData<T>,
}

impl<'a, I, V, T, U> IndexedGeometryView<'a, I, V, T, U> {
  pub fn new(v: &'a U, index: &'a Vec<I>) -> Self {
    Self {
      data: v,
      index,
      _v_phantom: PhantomData,
      _phantom: PhantomData,
    }
  }
}

impl<'a, I, V, T, U> AnyGeometry for IndexedGeometryView<'a, I, V, T, U>
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

impl<'a, I, V, T, U> AnyIndexGeometry for IndexedGeometryView<'a, I, V, T, U>
where
  V: Positioned<f32, 3>,
  T: IndexPrimitiveTopology<I, V>,
  T::Primitive: IndexedPrimitiveData<I, V>,
  U: GeometryDataContainer<V>,
{
  type IndexPrimitive = <T::Primitive as IndexedPrimitiveData<I, V>>::IndexIndicator;

  fn index_primitive_at(&self, primitive_index: usize) -> Self::IndexPrimitive {
    let index = primitive_index * T::STEP;
    T::Primitive::create_index_indicator(&self.index, index)
  }
}
