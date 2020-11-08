use crate::{ShaderGraph, ShaderGraphBindGroupBuilder, ShaderGraphBuilder};
use rendiation_ral::ShaderStage;

pub trait ShaderGraphProvider {
  fn build_graph() -> ShaderGraph;
}

pub trait ShaderGraphBuilderCreator {
  type ShaderGraphShaderInstance;
  fn create_builder() -> (ShaderGraphBuilder, Self::ShaderGraphShaderInstance);
}

pub trait ShaderGraphBindGroupItemProvider {
  type ShaderGraphBindGroupItemInstance;

  fn create_instance<'a>(
    name: &'static str,
    bindgroup_builder: &mut ShaderGraphBindGroupBuilder<'a>,
    stage: ShaderStage,
  ) -> Self::ShaderGraphBindGroupItemInstance;
}

pub trait ShaderGraphBindGroupProvider {
  type ShaderGraphBindGroupInstance;

  fn create_instance<'a>(
    bindgroup_builder: &mut ShaderGraphBindGroupBuilder<'a>,
  ) -> Self::ShaderGraphBindGroupInstance;
}

pub trait ShaderGraphGeometryProvider {
  type ShaderGraphGeometryInstance;

  fn create_instance(builder: &mut ShaderGraphBuilder) -> Self::ShaderGraphGeometryInstance;
}
