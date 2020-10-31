use crate::{ShaderGraphBackend, ShaderGraphBindGroupBuilder, ShaderGraphBuilder};
use rendiation_ral::{ShaderStage, UBOData};

pub trait ShaderGraphFactory<T: ShaderGraphBackend> {
  type ShaderGraphShaderInstance;
  fn create_builder(
    renderer: &T::Renderer,
  ) -> (ShaderGraphBuilder<T>, Self::ShaderGraphShaderInstance);
}

pub trait ShaderGraphBindGroupItemProvider<T: ShaderGraphBackend> {
  type ShaderGraphBindGroupItemInstance;

  fn create_instance<'a>(
    name: &'static str,
    bindgroup_builder: &mut ShaderGraphBindGroupBuilder<'a, T>,
    stage: ShaderStage,
  ) -> Self::ShaderGraphBindGroupItemInstance;
}

pub trait ShaderGraphBindGroupProvider<T: ShaderGraphBackend> {
  type ShaderGraphBindGroupInstance;

  fn create_instance<'a>(
    bindgroup_builder: &mut ShaderGraphBindGroupBuilder<'a, T>,
  ) -> Self::ShaderGraphBindGroupInstance;
}

pub trait ShaderGraphGeometryProvider<T: ShaderGraphBackend> {
  type ShaderGraphGeometryInstance;

  fn create_instance(builder: &mut ShaderGraphBuilder<T>) -> Self::ShaderGraphGeometryInstance;
}

pub trait ShaderGraphUBO<T: ShaderGraphBackend>:
  ShaderGraphBindGroupItemProvider<T> + UBOData
{
  // todo maybe return static ubo info
}
