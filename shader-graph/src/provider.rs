use crate::{
  ShaderGraphBindGroupBuilder, ShaderGraphBuilder, ShaderGraphNodeHandle, ShaderGraphNodeType,
};

pub trait ShaderGraphBindGroupItemProvider {
  type ShaderGraphBindGroupItemInstance;

  fn create_instance<'a>(
    name: &'static str,
    bindgroup_builder: &mut ShaderGraphBindGroupBuilder<'a>,
  ) -> Self::ShaderGraphBindGroupItemInstance;
}

pub struct ShaderGraphSampler;

impl ShaderGraphNodeType for ShaderGraphSampler {
  fn to_glsl_type() -> &'static str {
    "sampler"
  }
}

impl ShaderGraphBindGroupItemProvider for ShaderGraphSampler {
  type ShaderGraphBindGroupItemInstance = ShaderGraphNodeHandle<ShaderGraphSampler>;

  fn create_instance<'a>(
    name: &'static str,
    bindgroup_builder: &mut ShaderGraphBindGroupBuilder<'a>,
  ) -> Self::ShaderGraphBindGroupItemInstance {
    bindgroup_builder.uniform::<ShaderGraphSampler>(name)
  }
}

pub struct ShaderGraphTexture;

impl ShaderGraphNodeType for ShaderGraphTexture {
  fn to_glsl_type() -> &'static str {
    "texture2D"
  }
}

impl ShaderGraphBindGroupItemProvider for ShaderGraphTexture {
  type ShaderGraphBindGroupItemInstance = ShaderGraphNodeHandle<ShaderGraphTexture>;

  fn create_instance<'a>(
    name: &'static str,
    bindgroup_builder: &mut ShaderGraphBindGroupBuilder<'a>,
  ) -> Self::ShaderGraphBindGroupItemInstance {
    bindgroup_builder.uniform::<ShaderGraphTexture>(name)
  }
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
