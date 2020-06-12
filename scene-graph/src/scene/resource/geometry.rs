use crate::{Index, ResourceManager, SceneGraphBackEnd, ResouceWrap};
use std::{marker::PhantomData, ops::Range};

// pub trait Geometry<T: SceneGraphBackEnd> {
//   fn update_gpu(&mut self, renderer: &mut T::Renderer);
//   fn get_gpu_index_buffer(&self) -> & T::IndexBuffer;
//   fn get_gpu_vertex_buffer(&self, index: usize) -> & T::VertexBuffer;
//   fn vertex_buffer_count(&self) -> usize;
//   fn get_draw_range(&self) -> Range<u32>;
//   // fn get_bounding_local(&self) -> &BoundingData;
// }

// pub struct SceneGeometry<T: SceneGraphBackEnd> {
//   index: Index,
//   // pub data: Box<dyn Geometry<T>>,
//   pub gpu: SceneGeometryData<T>,
// }

pub struct SceneGeometryData<T: SceneGraphBackEnd>{
  pub draw_range: Range<u32>,
  pub index_buffer: Option<Index>,
  pub vertex_buffers: Vec<Index>,
  phantom: PhantomData<T>
}

impl<T: SceneGraphBackEnd> SceneGeometryData<T> {
  pub fn new() ->Self{
    Self{
      draw_range: 0..0,
      index_buffer: None,
      vertex_buffers: Vec::new(),
      phantom: PhantomData
    }
  }
}

// impl<T: SceneGraphBackEnd> SceneGeometry<T>{
//   pub fn index(&self) -> Index {
//     self.index
//   }

//   // pub fn gpu(&self) -> &T::UniformBuffer {
//   //   &self.gpu
//   // }

//   // pub fn gpu_mut(&mut self) -> &mut T::UniformBuffer {
//   //   &mut self.gpu
//   // }
// }

impl<T: SceneGraphBackEnd> ResourceManager<T> {
  pub fn add_geometry(&mut self, g: SceneGeometryData<T>) -> &mut ResouceWrap<SceneGeometryData<T>> {
    ResouceWrap::new_wrap(&mut self.geometries, g)
  }

  pub fn get_geometry_mut(&mut self, index: Index) -> &mut ResouceWrap<SceneGeometryData<T>> {
    self.geometries.get_mut(index).unwrap()
  }

  pub fn get_geometry(&self, index: Index) -> &ResouceWrap<SceneGeometryData<T>> {
    self.geometries.get(index).unwrap()
  }

  pub fn delete_geometry(&mut self, index: Index) {
    self.geometries.remove(index);
  }

  pub fn delete_geometry_with_buffers(&mut self, index: Index) {
    let geometry = self.geometries.get(index).unwrap().resource();
    if let Some(b) = geometry.index_buffer{
      self.index_buffers.remove(b);
    }
    for b in &geometry.vertex_buffers{
      self.vertex_buffers.remove(*b);
    }
    self.geometries.remove(index);
  }

  pub fn add_index_buffer(&mut self, g: T::IndexBuffer) -> &mut ResouceWrap<T::IndexBuffer> {
    ResouceWrap::new_wrap(&mut self.index_buffers, g)
  }

  pub fn get_index_buffer_mut(&mut self, index: Index) -> &mut ResouceWrap<T::IndexBuffer> {
    self.index_buffers.get_mut(index).unwrap()
  }

  pub fn get_index_buffer(&self, index: Index) -> &ResouceWrap<T::IndexBuffer> {
    self.index_buffers.get(index).unwrap()
  }

  pub fn delete_index_buffer(&mut self, index: Index) {
    self.index_buffers.remove(index);
  }

  pub fn add_vertex_buffer(&mut self, g: T::VertexBuffer) -> &mut ResouceWrap<T::VertexBuffer> {
    ResouceWrap::new_wrap(&mut self.vertex_buffers, g)
  }

  pub fn get_vertex_buffer_mut(&mut self, index: Index) -> &mut ResouceWrap<T::VertexBuffer> {
    self.vertex_buffers.get_mut(index).unwrap()
  }

  pub fn get_vertex_buffer(&self, index: Index) -> &ResouceWrap<T::VertexBuffer> {
    self.vertex_buffers.get(index).unwrap()
  }

  pub fn delete_vertex_buffer(&mut self, index: Index) {
    self.vertex_buffers.remove(index);
  }
}
