//! The actually geometry data container, define how we store the vertex

pub mod indexed_geometry;
pub mod indexed_geometry_view;
pub mod none_indexed_geometry;
pub mod none_indexed_geometry_view;

pub use indexed_geometry::*;
pub use indexed_geometry_view::*;
pub use none_indexed_geometry::*;
pub use none_indexed_geometry_view::*;

use std::{iter::FromIterator, ops::Index};

pub trait GeometryDataContainer<T>:
  AsRef<[T]> + Clone + Index<usize, Output = T> + FromIterator<T>
{
}

impl<T: Clone> GeometryDataContainer<T> for Vec<T> {}

pub trait AnyGeometry {
  type Primitive;

  fn draw_count(&self) -> usize;
  fn primitive_count(&self) -> usize;
  fn primitive_at(&self, primitive_index: usize) -> Self::Primitive;

  fn primitive_iter(&self) -> AnyGeometryIter<'_, Self>
  where
    Self: Sized,
  {
    AnyGeometryIter {
      geometry: &self,
      current: 0,
      count: self.primitive_count(),
    }
  }
}

pub struct AnyGeometryIter<'a, G> {
  geometry: &'a G,
  current: usize,
  count: usize,
}

impl<'a, G: AnyGeometry> Iterator for AnyGeometryIter<'a, G> {
  type Item = G::Primitive;

  #[inline(always)]
  fn next(&mut self) -> Option<Self::Item> {
    if self.current == self.count {
      return None;
    }
    let p = self.geometry.primitive_at(self.current);
    self.current += 1;
    Some(p)
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    let len = self.geometry.primitive_count() - self.current;
    (len, Some(len))
  }
}

impl<'a, G: AnyGeometry> ExactSizeIterator for AnyGeometryIter<'a, G> {
  #[inline(always)]
  fn len(&self) -> usize {
    self.geometry.primitive_count() - self.current
  }
}

pub trait AnyIndexGeometry: AnyGeometry {
  type IndexPrimitive;

  fn index_primitive_at(&self, primitive_index: usize) -> Self::IndexPrimitive;

  fn index_primitive_iter(&self) -> AnyIndexGeometryIter<'_, Self>
  where
    Self: Sized,
  {
    AnyIndexGeometryIter {
      geometry: &self,
      current: 0,
      count: self.primitive_count(),
    }
  }
}

pub struct AnyIndexGeometryIter<'a, G> {
  geometry: &'a G,
  current: usize,
  count: usize,
}

impl<'a, G: AnyIndexGeometry> Iterator for AnyIndexGeometryIter<'a, G> {
  type Item = G::IndexPrimitive;

  #[inline(always)]
  fn next(&mut self) -> Option<Self::Item> {
    if self.current == self.count {
      return None;
    }
    let p = self.geometry.index_primitive_at(self.current);
    self.current += 1;
    Some(p)
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    let len = self.geometry.primitive_count() - self.current;
    (len, Some(len))
  }
}

impl<'a, G: AnyIndexGeometry> ExactSizeIterator for AnyIndexGeometryIter<'a, G> {
  #[inline(always)]
  fn len(&self) -> usize {
    self.geometry.primitive_count() - self.current
  }
}
