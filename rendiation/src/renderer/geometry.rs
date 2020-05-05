use crate::geometry::primitive::*;
use crate::geometry::standard_geometry::StandardGeometry;
use crate::renderer::buffer::WGPUBuffer;
use crate::renderer::render_pass::WGPURenderPass;
use crate::renderer::WGPURenderer;
use crate::{scene::resource::Geometry, vertex::Vertex};

pub struct GPUGeometry<T: PrimitiveTopology = TriangleList> {
  geometry: StandardGeometry<T>,
  data_changed: bool,
  index_changed: bool,
  gpu_data: Option<WGPUBuffer>,
  gpu_index: Option<WGPUBuffer>,
}

impl<T: PrimitiveTopology + 'static> Geometry for GPUGeometry<T> {
  fn update_gpu(&mut self, renderer: &mut WGPURenderer) {
    if let Some(gpu_data) = &mut self.gpu_data {
      if self.data_changed {
        gpu_data.update(renderer, &self.geometry.data);
      }
    } else {
      self.gpu_data = Some(WGPUBuffer::new(
        renderer,
        &self.geometry.data,
        wgpu::BufferUsage::VERTEX,
      ))
    }

    if let Some(gpu_index) = &mut self.gpu_index {
      if self.index_changed {
        gpu_index.update(renderer, &self.geometry.index);
      }
    } else {
      self.gpu_index = Some(WGPUBuffer::new(
        renderer,
        &self.geometry.index,
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

  fn get_gpu_geometry_buffer(&self) -> &WGPUBuffer {
    if let Some(gpu_data) = &self.gpu_data {
      gpu_data
    } else {
      panic!("geometry not prepared")
    }
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
        gpu_data.update(renderer, &self.geometry.data);
      }
    } else {
      self.gpu_data = Some(WGPUBuffer::new(
        renderer,
        &self.geometry.data,
        wgpu::BufferUsage::VERTEX,
      ))
    }

    if let Some(gpu_index) = &mut self.gpu_index {
      if self.index_changed {
        gpu_index.update(renderer, &self.geometry.index);
      }
    } else {
      self.gpu_index = Some(WGPUBuffer::new(
        renderer,
        &self.geometry.index,
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
      .draw_indexed(0..self.geometry.get_full_count(), 0, 0..1);
  }
}
