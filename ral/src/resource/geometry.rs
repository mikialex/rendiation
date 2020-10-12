use crate::{
  GeometryHandle, GeometryProvider, IndexBufferHandle, RALBackend, ResourceManager, ResourceWrap,
  VertexBufferHandle,
};
use std::ops::Range;

pub struct GeometryResourceInstance<T: RALBackend, G: GeometryProvider<T>> {
  pub draw_range: Range<u32>,
  pub data: G::Instance,
  // pub index_buffer: Option<IndexBufferHandle<T>>,
  // pub vertex_buffers: Vec<VertexBufferHandle<T>>
}

impl<T: RALBackend, G: GeometryProvider<T>> GeometryResourceInstance<T, G> {
  pub fn new(data: G) -> Self {
    Self {
      draw_range: 0..0,
      data,
    }
  }
}

impl<T: RALBackend, G: GeometryProvider<T>> ResourceManager<T> {
  pub fn add_geometry(&mut self, g: GeometryResourceInstance<T, G>) -> GeometryHandle<T, G> {
    self.geometries.insert(g)
  }

  pub fn get_geometry_mut(
    &mut self,
    index: GeometryHandle<T, G>,
  ) -> &mut GeometryResourceInstance<T, G> {
    self.geometries.get_mut(index).unwrap()
  }

  pub fn get_geometry(&self, index: GeometryHandle<T, G>) -> &GeometryResourceInstance<T, G> {
    self.geometries.get(index).unwrap()
  }

  pub fn delete_geometry(&mut self, index: GeometryHandle<T, G>) {
    self.geometries.remove(index);
  }

  pub fn delete_geometry_with_buffers(&mut self, index: GeometryHandle<T, G>) {
    let geometry = self.geometries.get(index).unwrap();
    if let Some(b) = geometry.index_buffer {
      self.index_buffers.remove(b);
    }
    for b in &geometry.vertex_buffers {
      self.vertex_buffers.remove(*b);
    }
    self.geometries.remove(index);
  }

  pub fn add_index_buffer(&mut self, g: T::IndexBuffer) -> &mut ResourceWrap<T::IndexBuffer> {
    ResourceWrap::new_wrap(&mut self.index_buffers, g)
  }

  pub fn get_index_buffer_mut(
    &mut self,
    index: IndexBufferHandle<T>,
  ) -> &mut ResourceWrap<T::IndexBuffer> {
    self.index_buffers.get_mut(index).unwrap()
  }

  pub fn get_index_buffer(&self, index: IndexBufferHandle<T>) -> &ResourceWrap<T::IndexBuffer> {
    self.index_buffers.get(index).unwrap()
  }

  pub fn delete_index_buffer(&mut self, index: IndexBufferHandle<T>) {
    self.index_buffers.remove(index);
  }

  pub fn add_vertex_buffer(&mut self, g: T::VertexBuffer) -> &mut ResourceWrap<T::VertexBuffer> {
    ResourceWrap::new_wrap(&mut self.vertex_buffers, g)
  }

  pub fn get_vertex_buffer_mut(
    &mut self,
    index: VertexBufferHandle<T>,
  ) -> &mut ResourceWrap<T::VertexBuffer> {
    self.vertex_buffers.get_mut(index).unwrap()
  }

  pub fn get_vertex_buffer(&self, index: VertexBufferHandle<T>) -> &ResourceWrap<T::VertexBuffer> {
    self.vertex_buffers.get(index).unwrap()
  }

  pub fn delete_vertex_buffer(&mut self, index: VertexBufferHandle<T>) {
    self.vertex_buffers.remove(index);
  }
}

// pub struct GeometryPair<R: RALBackend, T: GeometryProvider<R>> {
//   data: T::Instance,
//   gpu: Option<R::>,
// }
