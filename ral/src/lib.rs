#![allow(clippy::new_without_default)]
#![allow(clippy::option_map_unit_fn)]

use std::ops::Range;

mod resource;
mod shader_info;
mod target_state;
mod viewport;
mod wgpu_reexport;

pub use resource::*;
pub use shader_info::*;
pub use target_state::*;
pub use viewport::*;
pub use wgpu_reexport::*;

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

  fn render_drawcall(
    drawcall: &Drawcall<Self>,
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
  const DESCRIPTOR: VertexBufferDescriptor<'static>;
}

/// should impl for geometry
pub trait VertexStateDescriptorProvider {
  fn create_descriptor() -> VertexStateDescriptor<'static>;
}

pub trait GeometryDescriptorProvider: VertexStateDescriptorProvider {
  fn get_primitive_topology() -> PrimitiveTopology;
}

pub trait BindGroupLayoutDescriptorProvider {
  fn create_descriptor() -> Vec<BindGroupLayoutEntry>;
}

pub trait BindGroupLayoutEntryProvider {
  fn create_layout_entry(binding: u32, visibility: ShaderStage) -> BindGroupLayoutEntry;
}

impl<T: UBOData> BindGroupLayoutEntryProvider for T {
  fn create_layout_entry(binding: u32, visibility: ShaderStage) -> BindGroupLayoutEntry {
    BindGroupLayoutEntry {
      binding,
      visibility,
      ty: BindingType::UniformBuffer {
        dynamic: false,
        min_binding_size: None, // todo investigate
      },
      count: None,
    }
  }
}

impl BindGroupLayoutEntryProvider for ShaderTexture {
  fn create_layout_entry(binding: u32, visibility: ShaderStage) -> BindGroupLayoutEntry {
    BindGroupLayoutEntry {
      binding,
      visibility,
      ty: BindingType::SampledTexture {
        multisampled: false,
        component_type: wgpu::TextureComponentType::Float,
        dimension: wgpu::TextureViewDimension::D2,
      },
      count: None,
    }
  }
}

impl BindGroupLayoutEntryProvider for ShaderSampler {
  fn create_layout_entry(binding: u32, visibility: ShaderStage) -> BindGroupLayoutEntry {
    BindGroupLayoutEntry {
      binding,
      visibility,
      ty: BindingType::Sampler { comparison: false },
      count: None,
    }
  }
}
