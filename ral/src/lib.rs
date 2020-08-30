// cal for Content abstraction layer

use std::{marker::PhantomData, ops::Range};

mod resource;
mod shader;
mod shading;
pub use resource::*;
pub use shader::*;
pub use shading::*;

pub trait RALBackend: 'static {
  type RenderTarget;
  type RenderPass;
  type Renderer;
  type Shading;
  type BindGroup;
  type IndexBuffer;
  type VertexBuffer;
  type UniformBuffer;
  type UniformValue;
  type Texture;
  type Sampler;
  type SampledTexture;

  fn create_shading(renderer: &mut Self::Renderer, des: &SceneShadingDescriptor) -> Self::Shading;
  fn dispose_shading(renderer: &mut Self::Renderer, shading: Self::Shading);

  fn create_uniform_buffer(renderer: &mut Self::Renderer, data: &[u8]) -> Self::UniformBuffer;
  fn dispose_uniform_buffer(renderer: &mut Self::Renderer, uniform: Self::UniformBuffer);
  fn update_uniform_buffer(
    renderer: &mut Self::Renderer,
    gpu: &mut Self::UniformBuffer,
    data: &[u8],
    range: Range<usize>,
  );

  fn create_index_buffer(renderer: &mut Self::Renderer, data: &[u8]) -> Self::IndexBuffer;

  fn create_vertex_buffer(
    renderer: &mut Self::Renderer,
    data: &[u8],
    layout: RALVertexBufferDescriptor,
  ) -> Self::VertexBuffer;
}

pub struct UniformBufferRef<'a, T: RALBackend, U: 'static + Sized> {
  pub ty: PhantomData<U>,
  pub data: (&'a T::UniformBuffer, Range<u64>),
}

pub trait BindGroupProvider<T: RALBackend>: 'static {
  fn create_bindgroup(
    &self,
    renderer: &T::Renderer,
    resources: &ShaderBindableResourceManager<T>,
  ) -> T::BindGroup;
  fn apply(&self, render_pass: &mut T::RenderPass, gpu_bindgroup: &T::BindGroup);
}

pub trait ShadingProvider<T: RALBackend>: 'static {
  fn create_shading(&self, renderer: &T::Renderer, resources: &BindGroupManager<T>) -> T::Shading;
  fn apply(&self, render_pass: &mut T::RenderPass, gpu_shading: &T::Shading);
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct AttributeTypeId(pub u64);

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct UniformTypeId(pub u64);

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct ParameterGroupTypeId(pub u64);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ShaderStage {
  Vertex,
  Fragment,
}
