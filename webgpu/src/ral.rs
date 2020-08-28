use crate::*;
use rendiation_ral::*;
use std::ops::Range;

impl RALBackend for WGPURenderer {
  type RenderTarget = WGPURenderPassBuilder<'static>;
  type Renderer = WGPURenderer;
  type Shading = WGPUPipeline;
  type BindGroup = WGPUBindGroup;
  type IndexBuffer = WGPUBuffer;
  type VertexBuffer = WGPUBuffer;
  type UniformBuffer = WGPUBuffer;
  type UniformValue = ();
  type Texture = WGPUTexture;
  type Sampler = WGPUSampler;
  type SampledTexture = ();

  fn create_shading(_renderer: &mut WGPURenderer, _des: &SceneShadingDescriptor) -> Self::Shading {
    todo!()
    // let vertex = load_glsl(&des.shader_descriptor.vertex_shader_str, ShaderStage::VERTEX);
    // let frag = load_glsl(&des.shader_descriptor.frag_shader_str, ShaderStage::FRAGMENT);
    // PipelineBuilder::new(renderer, vertex, frag)
    //   // .geometry(des)
    //   .build() // todo add bindgroup state stuff
  }
  fn dispose_shading(_renderer: &mut WGPURenderer, _shading: Self::Shading) {
    // just drop!
  }
  fn create_uniform_buffer(renderer: &mut WGPURenderer, data: &[u8]) -> Self::UniformBuffer {
    WGPUBuffer::new(
      renderer,
      data,
      wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
    )
  }
  fn dispose_uniform_buffer(_renderer: &mut Self::Renderer, _uniform: Self::UniformBuffer) {
    // just drop!
  }
  fn update_uniform_buffer(
    renderer: &mut Self::Renderer,
    gpu: &mut Self::UniformBuffer,
    data: &[u8],
    _range: Range<usize>, // todo
  ) {
    gpu.update(renderer, data);
  }

  fn create_index_buffer(renderer: &mut Self::Renderer, data: &[u8]) -> Self::IndexBuffer {
    WGPUBuffer::new(renderer, data, wgpu::BufferUsage::INDEX)
  }

  fn create_vertex_buffer(
    renderer: &mut Self::Renderer,
    data: &[u8],
    _layout: RALVertexBufferDescriptor, // so can we use this to add additional runtime check?
  ) -> Self::VertexBuffer {
    WGPUBuffer::new(renderer, data, wgpu::BufferUsage::VERTEX)
  }
}

pub fn shader_stage_convert(stage: rendiation_ral::ShaderStage) -> wgpu::ShaderStage {
  use rendiation_ral::ShaderStage::*;
  match stage {
    Vertex => wgpu::ShaderStage::VERTEX,
    Fragment => wgpu::ShaderStage::FRAGMENT,
  }
}
