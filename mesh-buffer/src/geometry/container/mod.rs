//! The actually geometry data container, define how we store the vertex

pub mod indexed_geometry;
pub mod indexed_geometry_view;
pub mod none_indexed_geometry;
pub mod none_indexed_geometry_view;

pub use indexed_geometry::*;
pub use indexed_geometry_view::*;
pub use none_indexed_geometry::*;
pub use none_indexed_geometry_view::*;
use rendiation_ral::*;

use std::{iter::FromIterator, ops::Index};

pub trait GeometryDataContainer<T>:
  AsRef<[T]> + Clone + Index<usize, Output = T> + FromIterator<T>
{
}

impl<T: Clone> GeometryDataContainer<T> for Vec<T> {}

pub trait RALGeometryDataContainer<T, R>: GeometryDataContainer<T>
where
  T: GeometryProvider,
  R: RAL,
{
  fn create_gpu(
    &self,
    resources: &mut ResourceManager<R>,
    renderer: &mut R::Renderer,
    instance: &mut GeometryResourceInstance<R, T>,
  );
}

impl<R, T> RALGeometryDataContainer<T, R> for Vec<T>
where
  R: RAL,
  T: GeometryProvider + Clone + VertexBufferDescriptorProvider + bytemuck::Pod,
{
  fn create_gpu(
    &self,
    resources: &mut ResourceManager<R>,
    renderer: &mut R::Renderer,
    instance: &mut GeometryResourceInstance<R, T>,
  ) {
    let vertex_buffer =
      R::create_vertex_buffer(renderer, bytemuck::cast_slice(self.as_ref()), T::DESCRIPTOR);
    instance.vertex_buffers = vec![resources.add_vertex_buffer(vertex_buffer).index()];
  }
}

pub trait AnyGeometry {
  type Primitive;

  fn primitive_count(&self) -> usize;
  fn primitive_at(&self, primitive_index: usize) -> Self::Primitive;

  fn as_ref_container<'a>(&'a self) -> AnyGeometryRefContainer<'a, Self>
  where
    Self: Sized,
  {
    AnyGeometryRefContainer { geometry: self }
  }
}

pub struct AnyGeometryRefContainer<'a, G> {
  pub geometry: &'a G, // todo deref
}

impl<'a, G: AnyGeometry> AnyGeometryRefContainer<'a, G> {
  pub fn primitive_iter(&self) -> AnyGeometryIter<'a, G> {
    AnyGeometryIter {
      geometry: &self.geometry,
      current: 0,
      count: self.geometry.primitive_count(),
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
  fn as_ref_index_container<'a>(&'a self) -> AnyIndexGeometryRefContainer<'a, Self>
  where
    Self: Sized,
  {
    AnyIndexGeometryRefContainer { geometry: self }
  }
}

pub struct AnyIndexGeometryRefContainer<'a, G> {
  geometry: &'a G,
}

pub struct AnyIndexGeometryIter<'a, G> {
  geometry: &'a G,
  current: usize,
  count: usize,
}

impl<'a, G: AnyIndexGeometry> AnyIndexGeometryRefContainer<'a, G> {
  pub fn index_primitive_iter(&self) -> AnyIndexGeometryIter<'a, G> {
    AnyIndexGeometryIter {
      geometry: &self.geometry,
      current: 0,
      count: self.geometry.primitive_count(),
    }
  }
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
