use crate::geometry::primitive::*;
use crate::geometry::standard_geometry::StandardGeometry;
use crate::renderer::buffer::WGPUBuffer;
use crate::renderer::WGPURenderer;
use crate::{scene::resource::Geometry, vertex::*, WGPURenderPass};
use std::ops::Range;

pub fn as_bytes<T>(vec: &[T]) -> &[u8] {
  unsafe{
    std::slice::from_raw_parts(
      (vec as *const [T]) as *const u8,
      ::std::mem::size_of::<T>() * vec.len(),
    )
  }
}

pub struct GPUGeometry<T: PrimitiveTopology = TriangleList> {
  geometry: StandardGeometry<T>,
  data_changed: bool,
  index_changed: bool,
  gpu_data: Option<[WGPUBuffer; 1]>,
  gpu_index: Option<WGPUBuffer>,
}

impl<T: PrimitiveTopology + 'static> Geometry for GPUGeometry<T> {
  fn update_gpu(&mut self, renderer: &mut WGPURenderer) {
    if let Some(gpu_data) = &mut self.gpu_data {
      if self.data_changed {
        gpu_data[0].update(renderer, as_bytes(&self.geometry.data));
      }
    } else {
      self.gpu_data = Some([WGPUBuffer::new(
        renderer,
        as_bytes(&self.geometry.data),
        wgpu::BufferUsage::VERTEX,
      )])
    }

    if let Some(gpu_index) = &mut self.gpu_index {
      if self.index_changed {
        gpu_index.update(renderer, as_bytes(&self.geometry.index));
      }
    } else {
      self.gpu_index = Some(WGPUBuffer::new(
        renderer,
        as_bytes(&self.geometry.index),
        wgpu::BufferUsage::INDEX,
      ))
    }
  }

  fn get_gpu_index_buffer(&self) -> &WGPUBuffer {
    if let Some(gpu_index) = &self.gpu_index {
      gpu_index
    } else {
      panic!("geometry not prepared")
    }
  }

  fn get_gpu_vertex_buffer(&self, index: usize) -> &WGPUBuffer {
    if let Some(gpu_data) = &self.gpu_data {
      &gpu_data[0]
    } else {
      panic!("geometry not prepared")
    }
  }

  fn get_draw_range(&self) -> Range<u32> {
    0..self.geometry.get_full_count()
  }
  fn vertex_buffer_count(&self) -> usize {
    1
  }
}

impl<T: PrimitiveTopology> From<StandardGeometry<T>> for GPUGeometry<T> {
  fn from(geometry: StandardGeometry<T>) -> Self {
    GPUGeometry {
      geometry,
      data_changed: true,
      index_changed: true,
      gpu_data: None,
      gpu_index: None,
    }
  }
}

impl From<(Vec<Vertex>, Vec<u16>)> for GPUGeometry {
  fn from(item: (Vec<Vertex>, Vec<u16>)) -> Self {
    StandardGeometry::new(item.0, item.1).into()
  }
}

impl GPUGeometry {
  pub fn mutate_data(&mut self) -> &mut Vec<Vertex> {
    self.data_changed = true;
    &mut self.geometry.data
  }

  pub fn mutate_index(&mut self) -> &mut Vec<u16> {
    self.index_changed = true;
    &mut self.geometry.index
  }

  pub fn update_gpu(&mut self, renderer: &mut WGPURenderer) {
    if let Some(gpu_data) = &mut self.gpu_data {
      if self.data_changed {
        gpu_data[0].update(renderer, as_bytes(&self.geometry.data));
      }
    } else {
      self.gpu_data = Some([WGPUBuffer::new(
        renderer,
        as_bytes(&self.geometry.data),
        wgpu::BufferUsage::VERTEX,
      )])
    }

    if let Some(gpu_index) = &mut self.gpu_index {
      if self.index_changed {
        gpu_index.update(renderer, as_bytes(&self.geometry.index));
      }
    } else {
      self.gpu_index = Some(WGPUBuffer::new(
        renderer,
        as_bytes(&self.geometry.index),
        wgpu::BufferUsage::INDEX,
      ))
    }
  }

  pub fn provide_geometry<'a,  'b: 'a>(&'b self, pass: &mut WGPURenderPass<'a>) {
    if let Some(gpu_data) = &self.gpu_data {
      pass
        .gpu_pass
        .set_vertex_buffer(0, gpu_data[0].get_gpu_buffer(), 0, 0);
    } else {
      panic!("geometry not prepared")
    }

    if let Some(gpu_index) = &self.gpu_index {
      pass
        .gpu_pass
        .set_index_buffer(gpu_index.get_gpu_buffer(), 0, 0);
    } else {
      panic!("geometry not prepared")
    }
  }

  pub fn render<'a,  'b: 'a>(&'b self, pass: &mut WGPURenderPass<'a>) {
    self.provide_geometry(pass);
    pass
      .gpu_pass
      .draw_indexed(0..self.geometry.get_full_count(), 0, 0..1);
  }
}
