use crate::{AttributeTypeId, ParameterGroupTypeId, UniformTypeId};
use std::{
  collections::hash_map::DefaultHasher,
  hash::{Hash, Hasher},
};
use wasm_bindgen::prelude::*;

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
  #[wasm_bindgen(skip)]
  pub vertex_shader_str: String, // new sal(shading abstraction layer) is in design, assume shader just works

  #[wasm_bindgen(skip)]
  pub frag_shader_str: String,

  #[wasm_bindgen(skip)]
  pub input_group: Vec<ShaderInputGroupDescriptor>,

  #[wasm_bindgen(skip)]
  pub attribute_inputs: Vec<RALVertexBufferDescriptor>,
}

impl SceneShaderDescriptor {
  pub fn input_group(&self) -> &Vec<ShaderInputGroupDescriptor> {
    &self.input_group
  }
  pub fn attribute_inputs(&self) -> &Vec<RALVertexBufferDescriptor> {
    &self.attribute_inputs
  }
}

#[wasm_bindgen]
impl SceneShaderDescriptor {
  #[wasm_bindgen(constructor)]
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
  pub fn push_attribute_input(&mut self, g: RALVertexBufferDescriptor) {
    self.attribute_inputs.push(g)
  }
}

#[wasm_bindgen]
pub struct ShaderInputGroupDescriptor {
  inputs: Vec<ShaderInputDescriptor>,
  id: ParameterGroupTypeId,
}

impl ShaderInputGroupDescriptor {
  pub fn inputs(&self) -> &Vec<ShaderInputDescriptor> {
    &self.inputs
  }
  pub fn id(&self) -> ParameterGroupTypeId {
    self.id
  }
}

#[wasm_bindgen]
impl ShaderInputGroupDescriptor {
  pub fn new(unique_name: &str) -> Self {
    Self {
      inputs: Vec::new(),
      id: ParameterGroupTypeId(to_hash(&unique_name)),
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
  id: UniformTypeId,
}

impl ShaderInputDescriptor {
  pub fn id(&self) -> UniformTypeId {
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
    let id = UniformTypeId(to_hash(&name));
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
pub struct RALVertexBufferDescriptor {
  pub byte_stride: i32,
  attributes: Vec<RALVertexAttributeBufferDescriptor>,
}

impl RALVertexBufferDescriptor {
  pub fn attributes(&self) -> &Vec<RALVertexAttributeBufferDescriptor> {
    &self.attributes
  }
}

#[wasm_bindgen]
pub struct RALVertexAttributeBufferDescriptor {
  name: String,
  id: AttributeTypeId,
  pub byte_offset: i32,
  pub size: i32,
  pub data_type: RALVertexAttributeDataType,
}

impl RALVertexAttributeBufferDescriptor {
  pub fn id(&self) -> AttributeTypeId {
    self.id
  }
  pub fn name(&self) -> &str {
    &self.name
  }
}

#[wasm_bindgen]
#[derive(Copy, Clone, Debug)]
pub enum RALVertexAttributeDataType {
  F32,
  U16,
  I16,
  I8,
  U8,
}
