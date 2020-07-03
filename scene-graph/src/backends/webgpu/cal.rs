use crate::{CALBackend, SceneShadingDescriptor, SceneUniform, WebGPUBackend, CALVertexBufferDescriptor, CALAttributeTypeId};
use rendiation::*;

impl CALBackend for WebGPUBackend {
  fn create_shading(_renderer: &mut WGPURenderer, _des: &SceneShadingDescriptor) -> Self::Shading {
    todo!()
  }
  fn dispose_shading(_renderer: &mut WGPURenderer, _shading: Self::Shading) {
    // just drop!
  }
  fn create_uniform_buffer(renderer: &mut WGPURenderer, des: SceneUniform) -> Self::UniformBuffer {
    WGPUBuffer::new(renderer, des.value.as_byte(), wgpu::BufferUsage::UNIFORM)
  }
  fn dispose_uniform_buffer(_renderer: &mut Self::Renderer, _uniform: Self::UniformBuffer) {
    // just drop!
  }

  fn create_index_buffer(renderer: &mut Self::Renderer, data: &[u8]) -> Self::IndexBuffer {
    WGPUBuffer::new(renderer, data, wgpu::BufferUsage::INDEX)
  }

  fn create_vertex_buffer(
    renderer: &mut Self::Renderer,
    data: &[u8],
    _input_id: CALAttributeTypeId,
    _layout: CALVertexBufferDescriptor, // so can we use this to add additional runtime check?
  ) -> Self::VertexBuffer {
    WGPUBuffer::new(renderer, data, wgpu::BufferUsage::VERTEX)
  }
}
