use crate::{CALBackend, SceneShadingDescriptor, SceneUniform};
use rendiation::*;

struct WebGPUCALBackend {}

impl CALBackend for WebGPUCALBackend {
  type Renderer = WGPURenderer;
  type Shading = WGPUPipeline;
  fn create_shading(renderer: &mut WGPURenderer, _des: &SceneShadingDescriptor) -> Self::Shading {

    todo!()
  }
  fn dispose_shading(renderer: &mut WGPURenderer, _shading: Self::Shading) {
    // just drop!
  }
  type Uniform = WGPUBuffer;
  fn create_uniform_buffer(renderer: &mut WGPURenderer, _des: SceneUniform) -> Self::Uniform {
    todo!()
  }

  type IndexBuffer = WGPUBuffer;
  fn create_index_buffer(renderer: &mut Self::Renderer, data: &[u8]) -> Self::IndexBuffer{
    WGPUBuffer::new(
      renderer,
      data,
      wgpu::BufferUsage::INDEX,
    )
  }

  type VertexBuffer = WGPUBuffer;
  fn create_vertex_buffer(renderer: &mut Self::Renderer, data: &[u8]) -> Self::VertexBuffer{
    WGPUBuffer::new(
      renderer,
      data,
      wgpu::BufferUsage::VERTEX,
    )
  }
}
