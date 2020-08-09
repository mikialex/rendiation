//! The actually geometry data container, define how we store the vertex

pub mod indexed_geometry;
pub mod none_indexed_geometry;
use super::PrimitiveTopology;
pub use indexed_geometry::*;
pub use none_indexed_geometry::*;
use rendiation_math_entity::Positioned3D;
use std::{iter::FromIterator, ops::Index};

pub trait GeometryDataContainer<T>:
  AsRef<[T]> + Clone + Index<usize, Output = T> + FromIterator<T>
{
}

impl<T: Clone> GeometryDataContainer<T> for Vec<T> {}

pub trait AbstractGeometry: Sized {
  type Vertex: Positioned3D;
  type Topology: PrimitiveTopology<Self::Vertex>;

  fn wrap<'a>(&'a self) -> AbstractGeometryRef<'a, Self> {
    AbstractGeometryRef { wrapped: self }
  }

  fn primitive_iter<'a>(&'a self) -> AbstractPrimitiveIter<'a, Self> {
    AbstractPrimitiveIter(self)
  }
}

pub struct AbstractPrimitiveIter<'a, G: AbstractGeometry>(pub &'a G);

// wrapped struct for solve cross crate trait impl issue
pub struct AbstractGeometryRef<'a, G: AbstractGeometry> {
  pub wrapped: &'a G,
}

use std::ops::Deref;
impl<'a, G: AbstractGeometry> Deref for AbstractGeometryRef<'a, G> {
  type Target = G;
  fn deref(&self) -> &Self::Target {
    &self.wrapped
  }
}
