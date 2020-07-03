use std::{
  collections::hash_map::DefaultHasher,
  hash::{Hash, Hasher},
};
use wasm_bindgen::prelude::*;

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct CALAttributeTypeId(u64);

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct CALUniformTypeId(u64);

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct CALParameterGroupTypeId(u64);

fn to_hash<T>(obj: &T) -> u64
where
  T: Hash,
{
  let mut hasher = DefaultHasher::new(); // todo use rustc-hash for perf
  obj.hash(&mut hasher);
  hasher.finish()
}

#[wasm_bindgen]
pub struct SceneShaderDescriptor {
  vertex_shader_str: String, // new sal(shading abstraction layer) is in design, assume shader just works
  frag_shader_str: String,
  input_group: Vec<ShaderInputGroupDescriptor>,
  attribute_inputs: Vec<CALVertexBufferDescriptor>,
}

impl SceneShaderDescriptor {
  pub fn input_group(&self) -> &Vec<ShaderInputGroupDescriptor> {
    &self.input_group
  }
  pub fn attribute_inputs(&self) -> &Vec<CALVertexBufferDescriptor> {
    &self.attribute_inputs
  }
}

#[wasm_bindgen]
impl SceneShaderDescriptor {
  #[wasm_bindgen]
  pub fn new(vertex_shader_str: &str, frag_shader_str: &str) -> Self {
    Self {
      vertex_shader_str: vertex_shader_str.to_owned(),
      frag_shader_str: frag_shader_str.to_owned(),
      input_group: Vec::new(),
      attribute_inputs: Vec::new(),
    }
  }

  #[wasm_bindgen]
  pub fn vertex_shader_str_wasm(&self) -> String {
    self.vertex_shader_str.clone()
  }

  #[wasm_bindgen]
  pub fn frag_shader_str_wasm(&self) -> String {
    self.frag_shader_str.clone()
  }

  #[wasm_bindgen]
  pub fn push_input_group(&mut self, g: ShaderInputGroupDescriptor) {
    self.input_group.push(g)
  }

  #[wasm_bindgen]
  pub fn push_attribute_input(&mut self, g: CALVertexBufferDescriptor) {
    self.attribute_inputs.push(g)
  }
}

impl SceneShaderDescriptor {
  pub fn vertex_shader_str(&self) -> &str {
    &self.vertex_shader_str
  }

  pub fn frag_shader_str(&self) -> &str {
    &self.frag_shader_str
  }
}

#[wasm_bindgen]
pub struct ShaderInputGroupDescriptor {
  inputs: Vec<ShaderInputDescriptor>,
  id: CALParameterGroupTypeId,
}

impl ShaderInputGroupDescriptor {
  pub fn inputs(&self) -> &Vec<ShaderInputDescriptor> {
    &self.inputs
  }
}

#[wasm_bindgen]
impl ShaderInputGroupDescriptor {
  pub fn new(unique_name: &str) -> Self {
    Self {
      inputs: Vec::new(),
      id: CALParameterGroupTypeId(to_hash(&unique_name)),
    }
  }

  pub fn push_input(&mut self, des: ShaderInputDescriptor) {
    self.inputs.push(des);
  }
}

#[wasm_bindgen]
pub struct ShaderInputDescriptor {
  pub input_type: ShaderInputType,
  name: String,
  id: CALUniformTypeId,
}

impl ShaderInputDescriptor {
  pub fn id(&self) -> CALUniformTypeId {
    self.id
  }
  pub fn name(&self) -> &str {
    &self.name
  }
}

#[wasm_bindgen]
impl ShaderInputDescriptor {
  #[wasm_bindgen]
  pub fn new(name: String, input_type: ShaderInputType) -> Self {
    let id = CALUniformTypeId(to_hash(&name));
    Self {
      input_type,
      name,
      id,
    }
  }
}

#[wasm_bindgen]
#[derive(Copy, Clone, Debug)]
pub enum ShaderInputType {
  UniformBuffer,
}

#[wasm_bindgen]
pub struct CALVertexBufferDescriptor {
  pub byte_stride: i32,
  attributes: Vec<CALVertexAttributeBufferDescriptor>,
}

impl CALVertexBufferDescriptor {
  pub fn attributes(&self) -> &Vec<CALVertexAttributeBufferDescriptor> {
    &self.attributes
  }
}

#[wasm_bindgen]
pub struct CALVertexAttributeBufferDescriptor {
  name: String,
  id: CALAttributeTypeId,
  pub byte_offset: i32,
  pub size: i32,
  pub data_type: CALVertexAttributeDataType,
}

impl CALVertexAttributeBufferDescriptor {
  pub fn id(&self) -> CALAttributeTypeId {
    self.id
  }
  pub fn name(&self) -> &str {
    &self.name
  }
}

#[wasm_bindgen]
#[derive(Copy, Clone, Debug)]
pub enum CALVertexAttributeDataType {
  F32,
  U16,
  I16,
  I8,
  U8,
}
