use crate::renderer::pipeline::*;
use crate::renderer::render_pass::WGPURenderPass;
use crate::renderer::WGPURenderer;
use crate::renderer::buffer::WGPUBuffer;
use crate::vertex::Vertex;

pub struct StandardGeometry {
  data: Vec<Vertex>,
  data_changed: bool,
  index: Vec<u16>,
  index_changed: bool,
  gpu_data: WGPUBuffer,
  gpu_index: WGPUBuffer,
}

impl StandardGeometry {
  pub fn new(v: Vec<Vertex>, index: Vec<u16>, renderer: &WGPURenderer) -> Self {
    let gpu_data = renderer.create_vertex_buffer(&v);
    let gpu_index = renderer.create_index_buffer(&index);
    Self {
      data: v,
      data_changed: false, 
      index,
      index_changed: false,
      gpu_data,
      gpu_index,
    }
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
    if self.data_changed {
      self
        .gpu_data
        .update(&renderer.device, &mut renderer.encoder, &self.data);
    }
    if self.index_changed {
      self
        .gpu_index
        .update(&renderer.device, &mut renderer.encoder, &self.index);
    }
  }

  pub fn provide_gpu(&self, pass: &mut WGPURenderPass) {
    pass
      .gpu_pass
      .set_index_buffer(self.gpu_index.get_gpu_buffer(), 0);
    pass
      .gpu_pass
      .set_vertex_buffers(0, &[(self.gpu_data.get_gpu_buffer(), 0)]);
  }

  pub fn render(&self, pass: &mut WGPURenderPass) {
    self.provide_gpu(pass);
    pass
      .gpu_pass
      .draw_indexed(0..self.get_full_count(), 0, 0..1);
  }
}

impl<'a> GeometryProvider<'a> for StandardGeometry {
  fn get_geometry_layout_descriptor() -> Vec<wgpu::VertexBufferDescriptor<'a>> {
    vec![Vertex::get_buffer_layout_descriptor()]
  }
}
