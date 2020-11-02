// CAL for Content abstraction layer

use std::ops::Range;

mod resource;
mod viewport;
mod wgpu_re;

pub use resource::*;
pub use viewport::*;
pub use wgpu_re::*;

pub trait RAL: 'static + Sized {
  type RenderTarget;
  type RenderPass;
  type Renderer;
  type ShaderBuildSource;
  type Shading;
  type BindGroup;
  type IndexBuffer;
  type VertexBuffer;
  type UniformBuffer;
  type Texture;
  type Sampler;

  fn create_shading(renderer: &mut Self::Renderer, des: &Self::ShaderBuildSource) -> Self::Shading;
  fn dispose_shading(renderer: &mut Self::Renderer, shading: Self::Shading);
  fn apply_shading(pass: &mut Self::RenderPass, shading: &Self::Shading);
  fn apply_bindgroup(pass: &mut Self::RenderPass, index: usize, bindgroup: &Self::BindGroup);

  fn apply_vertex_buffer(pass: &mut Self::RenderPass, index: i32, vertex: &Self::VertexBuffer);
  fn apply_index_buffer(pass: &mut Self::RenderPass, index: &Self::IndexBuffer);

  fn create_uniform_buffer(renderer: &mut Self::Renderer, data: &[u8]) -> Self::UniformBuffer;
  fn dispose_uniform_buffer(renderer: &mut Self::Renderer, uniform: Self::UniformBuffer);
  fn update_uniform_buffer(
    renderer: &mut Self::Renderer,
    gpu: &mut Self::UniformBuffer,
    data: &[u8],
    range: Range<usize>,
  );

  fn create_index_buffer(renderer: &mut Self::Renderer, data: &[u8]) -> Self::IndexBuffer;
  fn dispose_index_buffer(renderer: &mut Self::Renderer, buffer: Self::IndexBuffer);

  fn create_vertex_buffer(
    renderer: &mut Self::Renderer,
    data: &[u8],
    layout: VertexBufferDescriptor<'static>,
  ) -> Self::VertexBuffer;
  fn dispose_vertex_buffer(renderer: &mut Self::Renderer, buffer: Self::VertexBuffer);

  fn set_viewport(pass: &mut Self::RenderPass, viewport: &Viewport);

  fn draw_indexed(pass: &mut Self::RenderPass, topology: PrimitiveTopology, range: Range<u32>);
  fn draw_none_indexed(pass: &mut Self::RenderPass, topology: PrimitiveTopology, range: Range<u32>);

  fn render_drawcall<G: GeometryProvider<Self>, SP: ShadingProvider<Self, Geometry = G>>(
    drawcall: &Drawcall<Self, G, SP>,
    pass: &mut Self::RenderPass,
    resources: &ResourceManager<Self>,
  );
}

#[derive(Copy, Clone)]
pub struct ShaderSampler;

#[derive(Copy, Clone)]
pub struct ShaderTexture;

/// should impl for vertex that geometry used
pub trait VertexBufferDescriptorProvider {
  fn create_descriptor() -> VertexBufferDescriptor<'static>;
}

/// should impl for geometry
pub trait VertexStateDescriptorProvider {
  fn create_descriptor() -> VertexStateDescriptor<'static>;
}

pub trait GeometryDescriptorProvider: VertexStateDescriptorProvider {
  fn get_primitive_topology() -> PrimitiveTopology;
}
