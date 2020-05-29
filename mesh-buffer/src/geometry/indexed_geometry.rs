use super::{IndexedPrimitiveIter, PrimitiveTopology, TriangleList};
use crate::vertex::Vertex;
use core::marker::PhantomData;
use rendiation_math_entity::PositionedPoint;

/// A indexed geometry that use vertex as primitive;
pub struct IndexedGeometry<V: PositionedPoint = Vertex, T: PrimitiveTopology<V> = TriangleList> {
  pub data: Vec<V>,
  pub index: Vec<u16>,
  _phantom: PhantomData<T>,
}

impl<V: PositionedPoint, T: PrimitiveTopology<V>> From<(Vec<V>, Vec<u16>)>
  for IndexedGeometry<V, T>
{
  fn from(item: (Vec<V>, Vec<u16>)) -> Self {
    IndexedGeometry::new(item.0, item.1)
  }
}

impl<V: PositionedPoint, T: PrimitiveTopology<V>> IndexedGeometry<V, T> {
  pub fn new(v: Vec<V>, index: Vec<u16>) -> Self {
    Self {
      data: v,
      index,
      _phantom: PhantomData,
    }
  }

  pub fn primitive_iter<'a>(&'a self) -> IndexedPrimitiveIter<'a, V, T::Primitive> {
    IndexedPrimitiveIter::new(&self.index, &self.data)
  }

  pub fn get_primitive_count(&self) -> u32 {
    self.index.len() as u32 / T::STRIDE as u32
  }

  pub fn get_full_count(&self) -> u32 {
    self.index.len() as u32
  }
}
