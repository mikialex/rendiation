// cal for Content abstraction layer

use std::any::Any;

mod shading;
mod shader;
pub use shading::*;
pub use shader::*;

pub trait CALBackend: SceneGraphBackend {
  fn create_shading(renderer: &mut Self::Renderer, des: &SceneShadingDescriptor) -> Self::Shading;
  fn dispose_shading(renderer: &mut Self::Renderer, shading: Self::Shading);

  fn create_uniform_buffer(renderer: &mut Self::Renderer, des: SceneUniform)
    -> Self::UniformBuffer;
  fn dispose_uniform_buffer(renderer: &mut Self::Renderer, uniform: Self::UniformBuffer);

  fn create_index_buffer(renderer: &mut Self::Renderer, data: &[u8]) -> Self::IndexBuffer;

  fn create_vertex_buffer(
    renderer: &mut Self::Renderer,
    data: &[u8],
    input_id: CALAttributeTypeId,
    layout: CALVertexBufferDescriptor,
  ) -> Self::VertexBuffer;
}

use crate::SceneGraphBackend;

pub struct SceneUniform {
  pub value: Box<dyn SceneUniformValue>,
}

pub trait SceneUniformValue: Any {
  fn as_any(&self) -> dyn Any;
  fn as_byte(&self) -> &[u8];
}
