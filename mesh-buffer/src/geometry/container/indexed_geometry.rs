use super::{
  super::{IndexedPrimitiveIter, PrimitiveTopology, TriangleList},
  AbstractGeometry, AbstractPrimitiveIter, GeometryDataContainer,
};
use crate::{
  geometry::{IndexedPrimitiveIterForPrimitiveOnly, PrimitiveData},
  vertex::Vertex,
};
use core::marker::PhantomData;
use rendiation_math_entity::Positioned3D;

impl<V, T, U> AbstractGeometry for IndexedGeometry<V, T, U>
where
  V: Positioned3D + 'static,
  T: PrimitiveTopology<V>,
  U: GeometryDataContainer<V>,
{
  type Vertex = V;
  type Topology = T;

  fn primitive_at(&self, primitive_index: usize) -> Option<<T as PrimitiveTopology<V>>::Primitive> {
    let stride = <<T as PrimitiveTopology<V>>::Primitive as PrimitiveData<V>>::DATA_STRIDE;
    let index = self.index.get(primitive_index * stride)?;
    Some(<<T as PrimitiveTopology<V>>::Primitive as PrimitiveData<
      V,
    >>::from_indexed_data(
      &self.index,
      self.data.as_ref(),
      *index as usize,
    ))
  }
}
pub trait IntoExactSizeIterator {
  type Item;
  type IntoIter: ExactSizeIterator<Item = Self::Item>;
  fn into_iter(self) -> Self::IntoIter;
}

impl<'a, V: Positioned3D + 'static, T: PrimitiveTopology<V>> IntoExactSizeIterator
  for AbstractPrimitiveIter<'a, IndexedGeometry<V, T>>
{
  type Item = T::Primitive;
  type IntoIter = IndexedPrimitiveIterForPrimitiveOnly<'a, V, Self::Item>;
  fn into_iter(self) -> Self::IntoIter {
    self.0.primitive_iter_no_index()
  }
}

impl<'a, V: Positioned3D + 'static, T: PrimitiveTopology<V>> IntoIterator
  for AbstractPrimitiveIter<'a, IndexedGeometry<V, T>>
{
  type Item = T::Primitive;
  type IntoIter = IndexedPrimitiveIterForPrimitiveOnly<'a, V, Self::Item>;
  fn into_iter(self) -> Self::IntoIter {
    self.0.primitive_iter_no_index()
  }
}

/// A indexed geometry that use vertex as primitive;
pub struct IndexedGeometry<
  V: Positioned3D = Vertex,
  T: PrimitiveTopology<V> = TriangleList,
  U: GeometryDataContainer<V> = Vec<V>,
> {
  pub data: U,
  pub index: Vec<u16>,
  _v_phantom: PhantomData<V>,
  _phantom: PhantomData<T>,
}

impl<V, T, U> From<(U, Vec<u16>)> for IndexedGeometry<V, T, U>
where
  V: Positioned3D,
  T: PrimitiveTopology<V>,
  U: GeometryDataContainer<V>,
{
  fn from(item: (U, Vec<u16>)) -> Self {
    IndexedGeometry::new(item.0, item.1)
  }
}

impl<V, T, U> IndexedGeometry<V, T, U>
where
  V: Positioned3D,
  T: PrimitiveTopology<V>,
  U: GeometryDataContainer<V>,
{
  pub fn new(v: U, index: Vec<u16>) -> Self {
    Self {
      data: v,
      index,
      _v_phantom: PhantomData,
      _phantom: PhantomData,
    }
  }

  pub fn primitive_iter<'a>(&'a self) -> IndexedPrimitiveIter<'a, V, T::Primitive> {
    IndexedPrimitiveIter::new(&self.index, self.data.as_ref())
  }

  pub fn primitive_iter_no_index<'a>(
    &'a self,
  ) -> IndexedPrimitiveIterForPrimitiveOnly<'a, V, T::Primitive> {
    IndexedPrimitiveIterForPrimitiveOnly(self.primitive_iter())
  }

  pub fn get_primitive_count(&self) -> u32 {
    self.index.len() as u32 / T::STRIDE as u32
  }

  pub fn get_full_count(&self) -> u32 {
    self.index.len() as u32
  }
}
