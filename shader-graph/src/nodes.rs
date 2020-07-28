use crate::ShaderFunction;
use std::{marker::PhantomData, sync::Arc};

#[derive(Debug, Copy, Clone)]
pub enum NodeType {
  Float,
  Vec2,
  Vec3,
  Vec4,
}

impl NodeType {
  pub fn to_glsl(self) -> &'static str {
    use NodeType::*;
    match self {
      Float => "float",
      Vec2 => "vec2",
      Vec3 => "vec3",
      Vec4 => "vec4",
    }
  }
}

pub struct ShaderGraphNode<T> {
  phantom: PhantomData<T>,
  pub data: ShaderGraphNodeData,
  pub node_type: NodeType,
}

impl<T> ShaderGraphNode<T> {
  pub fn new(data: ShaderGraphNodeData, node_type: NodeType) -> Self {
    Self {
      data,
      phantom: PhantomData,
      node_type,
    }
  }
  pub fn unwrap_as_input(&self) -> &ShaderGraphInputNode {
    match &self.data {
      ShaderGraphNodeData::Input(n) => n,
      _ => panic!("unwrap as input failed"),
    }
  }
}

pub enum ShaderGraphNodeData {
  Function(FunctionNode),
  Input(ShaderGraphInputNode),
}

pub struct ShaderGraphInputNode {
  pub node_type: ShaderGraphInputNodeType,
  pub name: String,
}

pub enum ShaderGraphInputNodeType {
  Uniform,
  Attribute,
}

pub struct FunctionNode {
  pub prototype: Arc<ShaderFunction>,
}
