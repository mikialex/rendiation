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
  type Sampler = WGPUSampler;

  fn create_shading(_renderer: &mut WGPURenderer, des: &Self::ShaderBuildSource) -> Self::Shading {
    WGPUPipeline::new(des)
  }
  fn dispose_shading(_renderer: &mut WGPURenderer, _shading: Self::Shading) {
    // just drop!
  }
  fn apply_shading(pass: &mut Self::RenderPass, shading: &Self::Shading) {
    pass.set_pipeline(unsafe { std::mem::transmute(shading) });
  }

  fn apply_bindgroup(pass: &mut Self::RenderPass, index: usize, bindgroup: &Self::BindGroup) {
    pass.set_bindgroup(index, unsafe { std::mem::transmute(bindgroup) });
  }

  fn apply_vertex_buffer(pass: &mut Self::RenderPass, index: i32, vertex: &Self::VertexBuffer) {
    pass.set_vertex_buffer(index as u32, unsafe { std::mem::transmute(vertex) });
  }
  fn apply_index_buffer(pass: &mut Self::RenderPass, index: &Self::IndexBuffer) {
    pass.set_index_buffer(unsafe { std::mem::transmute(index) });
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
    let geometry = resources.get_geometry(object.geometry);
    geometry.apply(pass, resources);

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
