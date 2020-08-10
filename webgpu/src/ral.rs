use crate::*;
use rendiation_ral::*;

impl RALBackend for WGPURenderer {
  type RenderTarget = WGPURenderPassBuilder<'static>;
  type Renderer = WGPURenderer;
  type Shading = WGPUPipeline;
  type ShadingParameterGroup = WGPUBindGroup;
  type IndexBuffer = WGPUBuffer;
  type VertexBuffer = WGPUBuffer;
  type UniformBuffer = WGPUBuffer;
  type UniformValue = ();
  type Texture = WGPUTexture;
  type Sampler = WGPUSampler;
  type SampledTexture = ();

  fn create_shading(renderer: &mut WGPURenderer, des: &SceneShadingDescriptor) -> Self::Shading {
    let vertex = load_glsl(&des.shader_descriptor.vertex_shader_str, ShaderType::Vertex);
    let frag = load_glsl(&des.shader_descriptor.frag_shader_str, ShaderType::Fragment);
    PipelineBuilder::new(renderer, vertex, frag)
      // .geometry(des)
      .build() // todo add bindgroup state stuff
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
    _layout: RALVertexBufferDescriptor, // so can we use this to add additional runtime check?
  ) -> Self::VertexBuffer {
    WGPUBuffer::new(renderer, data, wgpu::BufferUsage::VERTEX)
  }
}
