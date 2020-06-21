// Content abstraction layer
pub mod webgl;
pub mod webgpu;

use std::any::Any;

pub trait CALBackend {
  type Renderer;
  type Shading;
  fn create_shading(renderer: &mut Self::Renderer, des: &SceneShadingDescriptor) -> Self::Shading;
  fn dispose_shading(renderer: &mut Self::Renderer, shading: Self::Shading);

  type Uniform;
  fn create_uniform_buffer(renderer: &mut Self::Renderer, des: SceneUniform) -> Self::Uniform;
  fn dispose_uniform_buffer(renderer: &mut Self::Renderer, uniform: Self::Uniform);

  //   type Geometry;
  //   fn create_geometry(des: IndexedGeometry) -> Self::Geometry;
  type IndexBuffer;
  fn create_index_buffer(renderer: &mut Self::Renderer, data: &[u8]) -> Self::IndexBuffer;

  type VertexBuffer;
  fn create_vertex_buffer(renderer: &mut Self::Renderer, data: &[u8]) -> Self::VertexBuffer;
}

pub struct SceneShadingDescriptor {
  pub vertex_shader_str: String, // new sal(shading abstraction layer) is in design, assume shader just works
  pub frag_shader_str: String,
  // .. blend state stuff
  // .. target state stuff,

  // some think?
  // in opengl like backend, blend/target state is dynamically set on the ctx, target state is not be used at all.
  // in webgpu like backend, two mode:
  // 1. these state should explicitly and correctly provided and not perform runtime check, panic when not ok
  // 2. these state hashing to choose cached pso or create new in runtime, extra overhead and always ok.
  // but where should the strategy impl
}

impl SceneShadingDescriptor {
  pub fn new(vertex_shader_str: &str, frag_shader_str: &str) -> Self {
    Self {
      vertex_shader_str: vertex_shader_str.to_owned(),
      frag_shader_str: frag_shader_str.to_owned(),
    }
  }
}

pub struct SceneUniform {
  pub value: Box<dyn SceneUniformValue>,
}

pub trait SceneUniformValue: Any {
  fn as_any(&self) -> dyn Any;
  fn as_byte(&self) -> &[u8];
}
