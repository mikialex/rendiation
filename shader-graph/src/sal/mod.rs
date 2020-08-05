use crate::ShaderGraphBindGroupBuilder;

pub trait ShaderGraphUniformBuffer {
  type ShaderGraphUniformBufferInstance;
  
  fn create_instance<'a>(
    bindgroup_builder: &mut ShaderGraphBindGroupBuilder<'a>,
  ) -> Self::ShaderGraphUniformBufferInstance;
}
