use crate::geometry::primitive::PrimitiveTopology;
use crate::{geometry::*, vertex::Vertex};
use bytemuck::*;
use rendiation_math_entity::Positioned;
use rendiation_webgpu::*;
use std::ops::Range;

pub struct GPUGeometry<V: Positioned<f32, 3> = Vertex, T: PrimitiveTopology<V> = TriangleList> {
  geometry: IndexedGeometry<u16, V, T>,
  data_changed: bool,
  index_changed: bool,
  gpu_data: Option<[WGPUBuffer; 1]>,
  gpu_index: Option<WGPUBuffer>,
}

impl<V: Positioned<f32, 3>, T: PrimitiveTopology<V>> From<IndexedGeometry<u16, V, T>>
  for GPUGeometry<V, T>
{
  fn from(geometry: IndexedGeometry<u16, V, T>) -> Self {
    GPUGeometry {
      geometry,
      data_changed: true,
      index_changed: true,
      gpu_data: None,
      gpu_index: None,
    }
  }
}

impl<V: Positioned<f32, 3>, T: PrimitiveTopology<V>> From<(Vec<V>, Vec<u16>)>
  for GPUGeometry<V, T>
{
  fn from(item: (Vec<V>, Vec<u16>)) -> Self {
    IndexedGeometry::new(item.0, item.1).into()
  }
}

impl<V: Positioned<f32, 3> + Pod, T: PrimitiveTopology<V>> GPUGeometry<V, T> {
  pub fn mutate_data(&mut self) -> &mut Vec<V> {
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
        gpu_data[0].update(renderer, cast_slice(&self.geometry.data));
      }
    } else {
      self.gpu_data = Some([WGPUBuffer::new(
        renderer,
        cast_slice(&self.geometry.data),
        wgpu::BufferUsage::VERTEX,
      )])
    }

    if let Some(gpu_index) = &mut self.gpu_index {
      if self.index_changed {
        gpu_index.update(renderer, cast_slice(&self.geometry.index));
      }
    } else {
      self.gpu_index = Some(WGPUBuffer::new(
        renderer,
        cast_slice(&self.geometry.index),
        wgpu::BufferUsage::INDEX,
      ))
    }
  }

  pub fn get_vertex_buffer_unwrap(&self) -> &WGPUBuffer {
    if let Some(gpu_data) = &self.gpu_data {
      &gpu_data[0]
    } else {
      panic!("geometry not prepared")
    }
  }

  pub fn get_index_buffer_unwrap(&self) -> &WGPUBuffer {
    if let Some(gpu_index) = &self.gpu_index {
      &gpu_index
    } else {
      panic!("geometry not prepared")
    }
  }

  pub fn get_draw_range(&self) -> Range<u32> {
    0..self.geometry.data.len() as u32
  }

  pub fn provide_geometry<'a, 'b: 'a>(&'b self, pass: &mut WGPURenderPass<'a>) {
    pass.set_vertex_buffer(0, self.get_vertex_buffer_unwrap());
    pass.set_index_buffer(self.get_index_buffer_unwrap());
  }

  pub fn render<'a, 'b: 'a>(&'b self, pass: &mut WGPURenderPass<'a>) {
    self.provide_geometry(pass);
    pass
      .gpu_pass
      .draw_indexed(0..self.geometry.data.len() as u32, 0, 0..1);
  }
}
