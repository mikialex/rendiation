//! The actually geometry data container, define how we store the vertex

pub mod indexed_geometry;
pub mod indexed_iter;
pub mod none_indexed_geometry;
pub mod none_indexed_iter;

pub use indexed_geometry::*;
pub use indexed_iter::*;
pub use none_indexed_geometry::*;
pub use none_indexed_iter::*;
use rendiation_ral::{
  GeometryProvider, GeometryResourceInstance, ResourceManager, VertexBufferDescriptorProvider, RAL,
};

use super::PrimitiveTopology;
use rendiation_math_entity::Positioned3D;
use std::{iter::FromIterator, ops::Index};

pub trait GeometryDataContainer<T>:
  AsRef<[T]> + Clone + Index<usize, Output = T> + FromIterator<T>
{
}

pub trait RALGeometryDataContainer<T, R>: GeometryDataContainer<T>
where
  T: GeometryProvider<R>,
  R: RAL,
{
  fn create_gpu(
    &self,
    resources: &mut ResourceManager<R>,
    renderer: &mut R::Renderer,
    instance: &mut GeometryResourceInstance<R, T>,
  );
}

impl<T: Clone> GeometryDataContainer<T> for Vec<T> {}

impl<R, T> RALGeometryDataContainer<T, R> for Vec<T>
where
  R: RAL,
  T: GeometryProvider<R> + Clone + VertexBufferDescriptorProvider + bytemuck::Pod,
{
  fn create_gpu(
    &self,
    resources: &mut ResourceManager<R>,
    renderer: &mut R::Renderer,
    instance: &mut GeometryResourceInstance<R, T>,
  ) {
    let layout = T::create_descriptor();
    let vertex_buffer =
      R::create_vertex_buffer(renderer, bytemuck::cast_slice(self.as_ref()), layout);
    instance.vertex_buffers = vec![resources.add_vertex_buffer(vertex_buffer).index()];
  }
}

pub trait AbstractGeometry: Sized {
  type Vertex: Positioned3D;
  type Topology: PrimitiveTopology<Self::Vertex>;

  fn wrap<'a>(&'a self) -> AbstractGeometryRef<'a, Self> {
    AbstractGeometryRef { wrapped: self }
  }

  fn primitive_iter<'a>(&'a self) -> AbstractPrimitiveIter<'a, Self> {
    AbstractPrimitiveIter(self)
  }

  fn primitive_at(
    &self,
    primitive_index: usize,
  ) -> Option<<Self::Topology as PrimitiveTopology<Self::Vertex>>::Primitive>;
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

pub trait IntoExactSizeIterator {
  type Item;
  type IntoIter: ExactSizeIterator<Item = Self::Item>;
  fn into_iter(self) -> Self::IntoIter;
}
