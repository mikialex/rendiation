use crate::*;
use rendiation_ral::*;
use std::ops::Range;

impl RALBackend for WGPURenderer {
  type RenderTarget = Box<dyn RenderTargetAble>;
  type RenderPass = WGPURenderPass<'static>;
  type Renderer = WGPURenderer;
  type ShaderBuildSource = WGPUPipelineBuildSource;
  type Shading = WGPUPipeline;
  type BindGroup = WGPUBindGroup;
  type IndexBuffer = WGPUBuffer;
  type VertexBuffer = WGPUBuffer;
  type UniformBuffer = WGPUBuffer;
  type Texture = WGPUTexture;
  type TextureView = wgpu::TextureView;
  type Sampler = WGPUSampler;

  fn create_shading(_renderer: &mut WGPURenderer, des: &Self::ShaderBuildSource) -> Self::Shading {
    WGPUPipeline::new(des)
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

  fn dispose_index_buffer(_renderer: &mut Self::Renderer, _buffer: Self::IndexBuffer) {
    // just drop
  }

  fn create_vertex_buffer(
    renderer: &mut Self::Renderer,
    data: &[u8],
    _layout: RALVertexBufferDescriptor, // so can we use this to add additional runtime check?
  ) -> Self::VertexBuffer {
    WGPUBuffer::new(renderer, data, wgpu::BufferUsage::VERTEX)
  }

  fn dispose_vertex_buffer(_renderer: &mut Self::Renderer, _buffer: Self::VertexBuffer) {
    // just drop
  }

  fn render_object(
    object: &RenderObject<Self>,
    pass: &mut Self::RenderPass,
    resources: &ResourceManager<Self>,
  ) {
    let resources: &'static ResourceManager<Self> = unsafe { std::mem::transmute(resources) };

    // set shading
    resources
      .shadings
      .get_shading_boxed(object.shading)
      .apply(pass, resources);

    // set geometry
    let geometry = resources.get_geometry(object.geometry).resource();
    geometry.index_buffer.map(|b| {
      let index = resources.get_index_buffer(b);
      pass.set_index_buffer(index.resource());
    });
    for (i, vertex_buffer) in geometry.vertex_buffers.iter().enumerate() {
      let buffer = resources.get_vertex_buffer(*vertex_buffer);
      pass.set_vertex_buffer(i, buffer.resource());
    }

    // draw
    pass.draw_indexed(geometry.draw_range.clone())
  }
}

pub fn shader_stage_convert(stage: rendiation_ral::ShaderStage) -> wgpu::ShaderStage {
  use rendiation_ral::ShaderStage::*;
  match stage {
    Vertex => wgpu::ShaderStage::VERTEX,
    Fragment => wgpu::ShaderStage::FRAGMENT,
  }
}
