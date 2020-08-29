// cal for Content abstraction layer

use std::{any::Any, marker::PhantomData, ops::Range};

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
  // we never care what exact type is, just downcast and use any get method on it
  fn create_bindgroup(&self, renderer: &T::Renderer, resources: &dyn Any) -> T::BindGroup;
}

pub trait ShadingProvider<T: RALBackend>: 'static {
  // we never care what exact type is, just downcast and use any get method on it
  fn create_shading(&self, renderer: &T::Renderer, resources: &dyn Any) -> T::Shading;
  fn apply(&self, render_pass: &mut T::RenderPass, gpu_shading: &T::Shading);
}

// pub trait Renderable<T: RALBackend> {
//   fn render(renderer: );
// }

// pub struct RenderObject<T: RALBackend, S> {
//   pub shading_index: ShadingHandle<T, dyn Any>,
//   pub geometry_index: GeometryHandle<T>,
// }

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct AttributeTypeId(pub u64);

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct UniformTypeId(pub u64);

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct ParameterGroupTypeId(pub u64);

pub struct SceneUniform {
  pub value: Box<dyn SceneUniformValue>,
}

pub trait SceneUniformValue: Any {
  fn as_any(&self) -> dyn Any;
  fn as_byte(&self) -> &[u8];
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ShaderStage {
  Vertex,
  Fragment,
}
