use arena::Handle;
use wgpu::util::DeviceExt;

use crate::Renderer;
use std::any::Any;

// trait MeshBufferBackend: SceneBackend {
//   type VertexBuffer;
//   type VertexBufferGPU;
// }

// impl MeshBufferBackend for WebGPUScene {
//   type VertexBuffer = Box<dyn VertexBufferSource>;
//   type VertexBufferGPU = wgpu::Buffer;
// }

pub trait VertexBufferSource: 'static {
  fn as_any(&self) -> &dyn Any;
  fn as_bytes(&self) -> &[u8];
  fn get_layout(&self) -> wgpu::VertexBufferLayout;
}
pub struct VertexBuffer {
  data: Box<dyn VertexBufferSource>,
  gpu: Option<Handle<wgpu::Buffer>>,
}

impl VertexBuffer {
  pub fn new(data: impl VertexBufferSource) -> Self {
    let data = Box::new(data);
    Self { data, gpu: None }
  }

  pub fn update(&mut self, renderer: &mut Renderer) {
    let data = &self.data;
    self.gpu.get_or_insert_with(|| {
      let device = &renderer.device;
      let gpu = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: data.as_bytes(),
        usage: wgpu::BufferUsage::VERTEX,
      });
      renderer.buffers.insert(gpu)
    });
  }

  pub fn setup_pass<'a>(&self, renderer: &'a Renderer, pass: &mut wgpu::RenderPass<'a>, slot: u32) {
    let gpu = self.gpu.unwrap();
    let gpu = renderer.buffers.get(gpu).unwrap();
    pass.set_vertex_buffer(slot, gpu.slice(..));
  }
}

pub trait IndexBufferSource: 'static {
  fn as_any(&self) -> &dyn Any;
  fn as_bytes(&self) -> &[u8];
  fn index_format(&self) -> wgpu::IndexFormat;
}

pub struct IndexBuffer {
  data: Box<dyn IndexBufferSource>,
  gpu: Option<Handle<wgpu::Buffer>>,
}

impl IndexBuffer {
  pub fn new(data: impl IndexBufferSource) -> Self {
    let data = Box::new(data);
    Self { data, gpu: None }
  }

  pub fn update(&mut self, renderer: &mut Renderer) {
    let data = &self.data;
    self.gpu.get_or_insert_with(|| {
      let device = &renderer.device;
      let gpu = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: data.as_bytes(),
        usage: wgpu::BufferUsage::INDEX,
      });
      renderer.buffers.insert(gpu)
    });
  }

  pub fn setup_pass<'a>(&self, renderer: &'a Renderer, pass: &mut wgpu::RenderPass<'a>) {
    let gpu = self.gpu.unwrap();
    let gpu = renderer.buffers.get(gpu).unwrap();
    pass.set_index_buffer(gpu.slice(..), self.data.index_format());
  }
}
