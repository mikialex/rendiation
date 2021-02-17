use crate::{
  Node, ShaderFunctionMetaInfo, ShaderGraph, ShaderGraphNodeRawHandle,
  ShaderGraphNodeRawHandleUntyped, ShaderGraphNodeUntyped,
};
use rendiation_algebra::Vec2;
use rendiation_ral::{ShaderSampler, ShaderTexture};
use std::{any::TypeId, marker::PhantomData};

pub trait ShaderGraphNodeType: 'static + Copy {
  fn to_glsl_type() -> &'static str;
}

pub trait ShaderGraphConstableNodeType: 'static + Send + Sync {
  fn const_to_glsl(&self) -> String;
}

pub trait ShaderGraphNodeOrConst {
  type Output: ShaderGraphNodeType;
  fn to_node(&self, graph: &mut ShaderGraph) -> Node<Self::Output>;
}

impl<T: ShaderGraphConstableNodeType + ShaderGraphNodeType> ShaderGraphNodeOrConst for T {
  type Output = T;
  fn to_node(&self, graph: &mut ShaderGraph) -> Node<Self::Output> {
    let data = ShaderGraphNodeData::Const(Box::new(*self));
    let node = ShaderGraphNode::<T>::new(data);
    unsafe { graph.insert_node(node).handle.cast_type().into() }
  }
}

impl<T: ShaderGraphNodeType> ShaderGraphNodeOrConst for Node<T> {
  type Output = T;
  fn to_node(&self, _graph: &mut ShaderGraph) -> Node<Self::Output> {
    *self
  }
}

// this for not include samplers/textures as attributes
pub trait ShaderGraphAttributeNodeType: ShaderGraphNodeType {}

#[derive(Copy, Clone)]
pub struct AnyType {}

pub struct ShaderGraphNode<T: ShaderGraphNodeType> {
  phantom: PhantomData<T>,
  pub data: ShaderGraphNodeData,
  pub node_type: TypeId,
}

impl<T: ShaderGraphNodeType> ShaderGraphNode<T> {
  pub fn new(data: ShaderGraphNodeData) -> Self {
    Self {
      data,
      phantom: PhantomData,
      node_type: TypeId::of::<T>(),
    }
  }
  pub fn into_any(self) -> ShaderGraphNodeUntyped {
    unsafe { std::mem::transmute(self) }
  }
  pub fn into_typed(self) -> ShaderGraphNode<T> {
    unsafe { std::mem::transmute(self) }
  }

  pub fn unwrap_as_input(&self) -> &ShaderGraphInputNode {
    match &self.data {
      ShaderGraphNodeData::Input(n) => n,
      _ => panic!("unwrap as input failed"),
    }
  }

  pub fn unwrap_as_vary(&self) -> usize {
    match &self.data {
      ShaderGraphNodeData::Output(ShaderGraphOutput::Vary(n)) => *n,
      _ => panic!("unwrap as input failed"),
    }
  }
}

pub enum ShaderGraphNodeData {
  Function(FunctionNode),
  BuiltInFunction(&'static str),
  TextureSampling(TextureSamplingNode),
  Swizzle(&'static str),
  Operator(OperatorNode),
  Input(ShaderGraphInputNode),
  Output(ShaderGraphOutput),
  Const(Box<dyn ShaderGraphConstableNodeType>),
}

pub enum ShaderGraphOutput {
  Vary(usize),
  Frag(usize),
  Vert,
}

pub struct FunctionNode {
  pub prototype: &'static ShaderFunctionMetaInfo,
}

pub struct TextureSamplingNode {
  pub texture: ShaderGraphNodeRawHandle<ShaderTexture>,
  pub sampler: ShaderGraphNodeRawHandle<ShaderSampler>,
  pub position: ShaderGraphNodeRawHandle<Vec2<f32>>,
}

pub struct OperatorNode {
  pub left: ShaderGraphNodeRawHandleUntyped,
  pub right: ShaderGraphNodeRawHandleUntyped,
  pub operator: &'static str,
}

pub struct ShaderGraphInputNode {
  pub node_type: ShaderGraphInputNodeType,
  pub name: String,
}

pub enum ShaderGraphInputNodeType {
  Uniform,
  Attribute,
  Vary,
}
