use super::{
  super::{IndexedPrimitiveIter, PrimitiveTopology, TriangleList},
  AbstractGeometry, GeometryDataContainer,
};
use crate::{geometry::intersection::GeometryRayIntersection, vertex::Vertex};
use core::marker::PhantomData;
use rendiation_math_entity::Positioned3D;

impl<V, T, U> AbstractGeometry for IndexedGeometry<V, T, U>
where
  V: Positioned3D,
  T: PrimitiveTopology<V>,
  U: GeometryDataContainer<V>,
{
  type Vertex = V;
  type Topology = T;

  fn primitive_iter<'a>(
    &'a self,
  ) -> IndexedPrimitiveIter<
    'a,
    Self::Vertex,
    <Self::Topology as PrimitiveTopology<Self::Vertex>>::Primitive,
  > {
    self.primitive_iter()
  }
}
impl<V, T, U> GeometryRayIntersection for IndexedGeometry<V, T, U>
where
  V: Positioned3D,
  T: PrimitiveTopology<V>,
  U: GeometryDataContainer<V>,
{
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

  pub fn get_primitive_count(&self) -> u32 {
    self.index.len() as u32 / T::STRIDE as u32
  }

  pub fn get_full_count(&self) -> u32 {
    self.index.len() as u32
  }
}
