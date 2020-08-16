use crate::{
  ShaderGraphBindGroupBuilder, ShaderGraphBuilder, ShaderGraphNodeHandle, ShaderGraphNodeType,
};
use std::collections::HashMap;

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

pub trait ShaderGraphUBO: ShaderGraphBindGroupItemProvider {
  fn gen_header() -> &'static str;
}

/// use for compile time ubo field reflection by procedure macro;
pub struct UBOInfo {
  pub name: &'static str,
  pub fields: HashMap<&'static str, &'static str>,
  pub code_cache: String,
}

impl UBOInfo {
  pub fn new(name: &'static str) -> Self {
    Self {
      name,
      fields: HashMap::new(),
      code_cache: String::new(),
    }
  }
  pub fn add_field<T: ShaderGraphNodeType>(mut self, name: &'static str) -> Self {
    self.fields.insert(name, T::to_glsl_type());
    self
  }

  pub fn gen_code_cache(mut self) -> Self {
    self.code_cache = String::from("uniform ")
      + &self.name
      + " {\n"
      + self
        .fields
        .iter()
        .map(|(&name, &ty)| format!("  {} {}", ty, name))
        .collect::<Vec<_>>()
        .join(";\n")
        .as_str()
      + " \n}\n";

    self
  }
}
