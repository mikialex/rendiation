use crate::{
  modify_graph, Node, NodeUntyped, ShaderFunctionMetaInfo, ShaderGraphNodeRawHandleUntyped,
  ShaderGraphNodeUntyped, ShaderSampler, ShaderTexture,
};
use dyn_clone::DynClone;
use rendiation_algebra::Vec2;
use std::{any::TypeId, marker::PhantomData};

pub trait ShaderGraphNodeType: 'static + Copy {
  fn to_glsl_type() -> &'static str;
}

pub trait ShaderGraphConstableNodeType: 'static + Send + Sync + DynClone {
  fn const_to_glsl(&self) -> String;
}

impl<T> From<T> for Node<T>
where
  T: ShaderGraphConstableNodeType + ShaderGraphNodeType,
{
  fn from(input: T) -> Self {
    ShaderGraphNodeData::Const(ConstNode {
      data: Box::new(input),
    })
    .insert_graph()
  }
}

// this for not include samplers/textures as attributes
pub trait ShaderGraphAttributeNodeType: ShaderGraphNodeType {}

#[derive(Copy, Clone)]
pub struct AnyType;

impl<T> Node<T> {
  /// cast the underlayer handle to untyped, this cast is safe because
  /// we consider this a kind of up casting. Use this will reduce the
  /// unsafe code when we create ShaderGraphNodeData
  pub fn cast_untyped(&self) -> ShaderGraphNodeRawHandleUntyped {
    unsafe { self.handle.cast_type() }
  }

  pub fn cast_untyped_node(&self) -> NodeUntyped {
    self.cast_untyped().into()
  }
}

pub struct ShaderGraphNode<T> {
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

#[derive(Clone)]
pub enum ShaderGraphNodeData {
  Function(FunctionNode),
  BuiltInFunction {
    name: &'static str,
    parameters: Vec<ShaderGraphNodeRawHandleUntyped>,
  },
  TextureSampling(TextureSamplingNode),
  Swizzle {
    ty: &'static str,
    source: ShaderGraphNodeRawHandleUntyped,
  },
  Compose(Vec<ShaderGraphNodeRawHandleUntyped>),
  Operator(OperatorNode),
  Input(ShaderGraphInputNode),
  Output(ShaderGraphOutput),
  FieldGet {
    field_name: &'static str,
    struct_node: ShaderGraphNodeRawHandleUntyped,
  },
  StructConstruct {
    struct_id: TypeId,
    fields: Vec<ShaderGraphNodeRawHandleUntyped>,
  },
  Const(ConstNode),
}

pub struct ConstNode {
  pub data: Box<dyn ShaderGraphConstableNodeType>,
}

impl Clone for ConstNode {
  fn clone(&self) -> Self {
    Self {
      data: dyn_clone::clone_box(&*self.data),
    }
  }
}

impl ShaderGraphNodeData {
  pub fn insert_graph<T: ShaderGraphNodeType>(&self) -> Node<T> {
    modify_graph(|graph| {
      let node = ShaderGraphNode::<T>::new(self.clone());
      let result = graph.insert_node(node).handle;

      self.visit_dependency(|dep| {
        graph.nodes.connect_node(*dep, result);
      });

      unsafe { result.cast_type().into() }
    })
  }
  pub fn visit_dependency(&self, mut visitor: impl FnMut(&ShaderGraphNodeRawHandleUntyped)) {
    match self {
      ShaderGraphNodeData::Function(FunctionNode { parameters, .. }) => {
        parameters.iter().for_each(visitor)
      }
      ShaderGraphNodeData::BuiltInFunction { parameters, .. } => {
        parameters.iter().for_each(visitor)
      }
      ShaderGraphNodeData::TextureSampling(TextureSamplingNode {
        texture,
        sampler,
        position,
      }) => {
        visitor(&texture.cast_untyped());
        visitor(&sampler.cast_untyped());
        visitor(&position.cast_untyped());
      }
      ShaderGraphNodeData::Swizzle { source, .. } => visitor(source),
      ShaderGraphNodeData::Compose(source) => source.iter().for_each(visitor),
      ShaderGraphNodeData::Operator(OperatorNode { left, right, .. }) => {
        visitor(left);
        visitor(right);
      }
      ShaderGraphNodeData::Input(_) => {}
      ShaderGraphNodeData::Output(_) => {} // is this kind of node valid??
      ShaderGraphNodeData::FieldGet { struct_node, .. } => visitor(struct_node),
      ShaderGraphNodeData::StructConstruct { struct_id, fields } => fields.iter().for_each(visitor),
      ShaderGraphNodeData::Const(_) => {}
    }
  }
}

#[derive(Clone)]
pub enum ShaderGraphOutput {
  Vary(usize),
  Frag(usize),
  Vert,
}

#[derive(Clone)]
pub struct FunctionNode {
  pub prototype: &'static ShaderFunctionMetaInfo,
  pub parameters: Vec<ShaderGraphNodeRawHandleUntyped>,
}

#[derive(Clone)]
pub struct TextureSamplingNode {
  pub texture: Node<ShaderTexture>,
  pub sampler: Node<ShaderSampler>,
  pub position: Node<Vec2<f32>>,
}

#[derive(Clone)]
pub struct OperatorNode {
  pub left: ShaderGraphNodeRawHandleUntyped,
  pub right: ShaderGraphNodeRawHandleUntyped,
  pub operator: &'static str,
}

#[derive(Clone)]
pub struct ShaderGraphInputNode {
  pub node_type: ShaderGraphInputNodeType,
  pub name: String,
}

#[derive(Clone)]
pub enum ShaderGraphInputNodeType {
  Uniform,
  Attribute,
  Vary,
}
