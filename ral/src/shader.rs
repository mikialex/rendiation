use std::{
  collections::hash_map::DefaultHasher,
  hash::{Hash, Hasher},
};
use wasm_bindgen::prelude::*;

pub fn to_hash<T>(obj: &T) -> u64
where
  T: Hash,
{
  let mut hasher = DefaultHasher::new(); // todo use rustc-hash for perf
  obj.hash(&mut hasher);
  hasher.finish()
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
  pub byte_offset: i32,
  pub size: i32,
  pub data_type: RALVertexAttributeDataType,
}

impl RALVertexAttributeBufferDescriptor {
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
