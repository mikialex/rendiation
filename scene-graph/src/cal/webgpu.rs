use crate::{CALBackend, SceneShadingDescriptor, SceneUniform};
use rendiation::*;

struct WebGPUCALBackend {}

impl CALBackend for WebGPUCALBackend {
  type Renderer = WGPURenderer;
  type Shading = WGPUPipeline;
  fn create_shading(_renderer: &mut WGPURenderer, _des: &SceneShadingDescriptor) -> Self::Shading {
    todo!()
  }
  fn dispose_shading(_renderer: &mut WGPURenderer, _shading: Self::Shading) {
    // just drop!
  }
  type Uniform = WGPUBuffer;
  fn create_uniform_buffer(renderer: &mut WGPURenderer, des: SceneUniform) -> Self::Uniform {
    WGPUBuffer::new(renderer, des.value.as_byte(), wgpu::BufferUsage::UNIFORM)
  }
  fn dispose_uniform_buffer(_renderer: &mut Self::Renderer, _uniform: Self::Uniform) {
    // just drop!
  }

  type IndexBuffer = WGPUBuffer;
  fn create_index_buffer(renderer: &mut Self::Renderer, data: &[u8]) -> Self::IndexBuffer {
    WGPUBuffer::new(renderer, data, wgpu::BufferUsage::INDEX)
  }

  type VertexBuffer = WGPUBuffer;
  fn create_vertex_buffer(renderer: &mut Self::Renderer, data: &[u8]) -> Self::VertexBuffer {
    WGPUBuffer::new(renderer, data, wgpu::BufferUsage::VERTEX)
  }
}
