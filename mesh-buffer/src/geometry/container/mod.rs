//! The actually geometry data container, define how we store the vertex

pub mod indexed_geometry;
pub mod none_indexed_geometry;
use super::{IndexedPrimitiveIter, PrimitiveTopology};
pub use indexed_geometry::*;
pub use none_indexed_geometry::*;
use rendiation_math_entity::Positioned3D;
use std::{iter::FromIterator, ops::Index};

pub trait GeometryDataContainer<T>:
  AsRef<[T]> + Clone + Index<usize, Output = T> + FromIterator<T>
{
}

impl<T: Clone> GeometryDataContainer<T> for Vec<T> {}

pub trait AbstractGeometry {
  type Vertex: Positioned3D;
  type Topology: PrimitiveTopology<Self::Vertex>;

  fn primitive_iter<'a>(
    &'a self,
  ) -> IndexedPrimitiveIter<
    'a,
    Self::Vertex,
    <Self::Topology as PrimitiveTopology<Self::Vertex>>::Primitive,
  >;
}
