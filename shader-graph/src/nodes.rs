use crate::ShaderFunction;
use std::{marker::PhantomData, sync::Arc};

pub enum NodeType {
  Float,
  Vec2,
  Vec3,
  Vec4,
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
