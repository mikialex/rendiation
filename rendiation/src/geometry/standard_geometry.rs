use crate::primitive::*;
use crate::renderer::buffer::WGPUBuffer;
use crate::renderer::pipeline::*;
use crate::renderer::render_pass::WGPURenderPass;
use crate::renderer::WGPURenderer;
use crate::vertex::vertex;
use crate::vertex::Vertex;
use core::marker::PhantomData;

pub fn quad_maker() -> (Vec<Vertex>, Vec<u16>) {
  let data = [
    vertex([-1.0, -1.0, 0.0], [-1.0, -1.0, 1.0], [0.0, 1.0]),
    vertex([-1.0, 1.0, 0.0], [-1.0, -1.0, 1.0], [0.0, 0.0]),
    vertex([1.0, 1.0, 0.0], [-1.0, -1.0, 1.0], [1.0, 0.0]),
    vertex([1.0, -1.0, 0.0], [-1.0, -1.0, 1.0], [1.0, 1.0]),
  ];
  let index = [0, 2, 1, 2, 0, 3];
  (data.to_vec(), index.to_vec())
}

/// A indexed geometry that use vertex as primitive;
pub struct StandardGeometry<T: PrimitiveTopology = TriangleList> {
  data: Vec<Vertex>,
  index: Vec<u16>,
  _phantom: PhantomData<T>,
}

impl From<(Vec<Vertex>, Vec<u16>)> for StandardGeometry {
  fn from(item: (Vec<Vertex>, Vec<u16>)) -> Self {
    StandardGeometry::new(item.0, item.1)
  }
}

impl<T: PrimitiveTopology> StandardGeometry<T> {
  pub fn new(v: Vec<Vertex>, index: Vec<u16>) -> Self {
    Self {
      data: v,
      data_changed: false,
      index,
      index_changed: false,
      gpu_data: None,
      gpu_index: None,
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

  pub fn get_data(&self) -> &Vec<Vertex> {
    &self.data
  }

  pub fn get_index(&self) -> &Vec<u16> {
    &self.index
  }

  pub fn mutate_data(&mut self) -> &mut Vec<Vertex> {
    self.data_changed = true;
    &mut self.data
  }

  pub fn mutate_index(&mut self) -> &mut Vec<u16> {
    self.index_changed = true;
    &mut self.index
  }

  pub fn update_gpu(&mut self, renderer: &mut WGPURenderer) {
    if let Some(gpu_data) = &mut self.gpu_data {
      if self.data_changed {
        gpu_data.update(renderer, &self.data);
      }
    } else {
      self.gpu_data = Some(WGPUBuffer::new(
        renderer,
        &self.data,
        wgpu::BufferUsage::VERTEX,
      ))
    }

    if let Some(gpu_index) = &mut self.gpu_index {
      if self.index_changed {
        gpu_index.update(renderer, &self.index);
      }
    } else {
      self.gpu_index = Some(WGPUBuffer::new(
        renderer,
        &self.index,
        wgpu::BufferUsage::INDEX,
      ))
    }
  }

  pub fn provide_geometry(&self, pass: &mut WGPURenderPass) {
    if let Some(gpu_data) = &self.gpu_data {
      pass
        .gpu_pass
        .set_vertex_buffers(0, &[(gpu_data.get_gpu_buffer(), 0)]);
    } else {
      panic!("geometry not prepared")
    }

    if let Some(gpu_index) = &self.gpu_index {
      pass
        .gpu_pass
        .set_index_buffer(gpu_index.get_gpu_buffer(), 0);
    } else {
      panic!("geometry not prepared")
    }
  }

  pub fn render(&self, pass: &mut WGPURenderPass) {
    self.provide_geometry(pass);
    pass
      .gpu_pass
      .draw_indexed(0..self.get_full_count(), 0, 0..1);
  }
}

impl<'a, T: PrimitiveTopology> GeometryProvider for StandardGeometry<T> {
  fn get_geometry_layout_descriptor() -> Vec<wgpu::VertexBufferDescriptor<'static>> {
    vec![Vertex::get_buffer_layout_descriptor()]
  }

  fn get_primitive_topology() -> wgpu::PrimitiveTopology {
    T::WGPU_ENUM
  }

  fn get_index_format() -> wgpu::IndexFormat {
    wgpu::IndexFormat::Uint16
  }
}
