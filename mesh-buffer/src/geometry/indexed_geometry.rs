use core::marker::PhantomData;
use super::{TriangleList, IndexedPrimitiveIter, PrimitiveTopology};
use crate::vertex::Vertex;

/// A indexed geometry that use vertex as primitive;
pub struct IndexedGeometry<T = TriangleList, V = Vertex>
where
  T: PrimitiveTopology,
{
  pub data: Vec<V>,
  pub index: Vec<u16>,
  _phantom: PhantomData<T>,
}

impl From<(Vec<Vertex>, Vec<u16>)> for IndexedGeometry {
  fn from(item: (Vec<Vertex>, Vec<u16>)) -> Self {
    IndexedGeometry::new(item.0, item.1)
  }
}

impl<T: PrimitiveTopology> IndexedGeometry<T> {
  pub fn new(v: Vec<Vertex>, index: Vec<u16>) -> Self {
    Self {
      data: v,
      index,
      _phantom: PhantomData,
    }
  }

  pub fn primitive_iter<'a>(&'a self) -> IndexedPrimitiveIter<'a, T::Primitive> {
    IndexedPrimitiveIter::new(&self.index, &self.data)
  }

  pub fn get_primitive_count(&self) -> u32 {
    self.index.len() as u32 / T::STRIDE as u32
  }

  pub fn get_full_count(&self) -> u32 {
    self.index.len() as u32
  }
}
