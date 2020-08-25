use crate::{
  ShaderFunction, ShaderGraphNodeRawHandle, ShaderGraphNodeUntyped, ShaderGraphSampler,
  ShaderGraphTexture,
};
use rendiation_math::Vec2;
use std::{any::TypeId, marker::PhantomData, sync::Arc};

pub trait ShaderGraphNodeType: 'static + Copy {
  fn to_glsl_type() -> &'static str;
}

pub trait ShaderGraphConstableNodeType: 'static + Send + Sync {
  fn const_to_glsl(&self) -> String;
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
  pub fn to_any(self) -> ShaderGraphNodeUntyped {
    unsafe { std::mem::transmute(self) }
  }
  pub fn from_any(self) -> ShaderGraphNode<T> {
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
  TextureSampling(TextureSamplingNode),
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
  pub prototype: Arc<ShaderFunction>,
}

pub struct TextureSamplingNode {
  pub texture: ShaderGraphNodeRawHandle<ShaderGraphTexture>,
  pub sampler: ShaderGraphNodeRawHandle<ShaderGraphSampler>,
  pub position: ShaderGraphNodeRawHandle<Vec2<f32>>,
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
