use crate::*;
use rendiation_ral::*;
use std::ops::Range;

pub struct WebGPU;

impl RAL for WebGPU {
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
    _layout: rendiation_ral::VertexBufferLayout<'static>, // so can we use this to add additional runtime check?
  ) -> Self::VertexBuffer {
    WGPUBuffer::new(renderer, data, wgpu::BufferUsage::VERTEX)
  }

  fn dispose_vertex_buffer(_renderer: &mut Self::Renderer, _buffer: Self::VertexBuffer) {
    // just drop
  }

  fn set_viewport(pass: &mut Self::RenderPass, viewport: &Viewport) {
    pass.use_viewport(&viewport);
  }

  fn draw_indexed(
    pass: &mut Self::RenderPass,
    _: rendiation_ral::PrimitiveTopology,
    range: Range<u32>,
  ) {
    pass.draw_indexed(range)
  }
  fn draw_none_indexed(
    pass: &mut Self::RenderPass,
    _: rendiation_ral::PrimitiveTopology,
    range: Range<u32>,
  ) {
    pass.draw(range)
  }

  fn render_drawcall(
    drawcall: &Drawcall<Self>,
    pass: &mut Self::RenderPass,
    resources: &ResourceManager<Self>,
  ) {
    let resources: &'static ResourceManager<Self> = unsafe { std::mem::transmute(resources) };

    let (shading, geometry) = resources.get_resource(drawcall);
    pass.current_topology = ral_topology_to_webgpu_topology(geometry.get_topology()); // todo

    // set shading
    shading.apply(pass, resources);

    // set geometry
    geometry.apply(pass, resources);

    // draw
    geometry.draw(pass);
  }
}

fn ral_topology_to_webgpu_topology(
  t: rendiation_ral::PrimitiveTopology,
) -> wgpu::PrimitiveTopology {
  use rendiation_ral::PrimitiveTopology::*;
  match t {
    TriangleList => wgpu::PrimitiveTopology::TriangleList,
    _ => panic!("not support"),
  }
}

pub trait WGPUBindgroupItem<'a> {
  type Type;
  fn to_binding(item: Self::Type) -> WGPUBinding<'a>;
}

pub trait WGPUUBOData: UBOData {}

impl<'a, T: WGPUUBOData + 'static> WGPUBindgroupItem<'a> for T {
  type Type = UniformBufferRef<'a, WebGPU, T>;
  fn to_binding(item: Self::Type) -> WGPUBinding<'a> {
    WGPUBinding::BindBuffer(item.gpu)
  }
}

impl<'a> WGPUBindgroupItem<'a> for ShaderTexture {
  type Type = &'a WGPUTexture;
  fn to_binding(item: Self::Type) -> WGPUBinding<'a> {
    WGPUBinding::BindTexture(item.view())
  }
}

impl<'a> WGPUBindgroupItem<'a> for ShaderSampler {
  type Type = &'a WGPUSampler;
  fn to_binding(item: Self::Type) -> WGPUBinding<'a> {
    WGPUBinding::BindSampler(item)
  }
}

#[cfg(feature = "shadergraph")]
use rendiation_shadergraph::ShaderGraph;
#[cfg(feature = "shadergraph")]
pub fn convert_build_source(graph: &ShaderGraph) -> WGPUPipelineBuildSource {
  let compiled = graph.compile();

  WGPUPipelineBuildSource {
    vertex_shader: load_glsl(compiled.vertex_shader, rendiation_ral::ShaderStage::VERTEX),
    frag_shader: load_glsl(compiled.frag_shader, rendiation_ral::ShaderStage::FRAGMENT),
    shader_interface_info: compiled.shader_interface_info,
  }
}
