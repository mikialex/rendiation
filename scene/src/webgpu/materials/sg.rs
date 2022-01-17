use std::{
  any::{Any, TypeId},
  collections::HashMap,
  marker::PhantomData,
  rc::Rc,
};

use arena_graph::{ArenaGraph, ArenaGraphNodeHandle};
use rendiation_algebra::*;

pub struct ShaderBuilder {
  vertex: ShaderGraph,
  fragment: ShaderGraph,
}

impl ShaderBuilder {
  pub fn query<T: ShaderValue>(&self) -> Option<Node<T::ValueType>> {
    todo!()
  }

  pub fn register<T: ShaderValue>(&mut self, node: Node<T::ValueType>) {
    todo!()
  }

  pub fn compile(&self) -> String {
    todo!()
  }
}

#[derive(Default, Clone)]
pub struct ShaderGraph {
  graph: ArenaGraph<NodeData>,
  uniforms: Vec<NodeUnTyped>,
  input: Vec<NodeUnTyped>,
  output: Vec<NodeUnTyped>,
  registered_value: HashMap<TypeId, NodeUnTyped>,
}

pub type NodeUnTyped = ArenaGraphNodeHandle<NodeData>;

pub struct Node<T> {
  ty: PhantomData<T>,
  pub handle: ArenaGraphNodeHandle<NodeData>,
}

// pub fn connect(builder: &mut ShaderBuilder, nodes:)

pub struct ShaderFunction {
  code: &'static str,
}

#[derive(Clone)]
pub enum NodeData {
  FunctionCall(Rc<ShaderFunction>),
  Operator,
  Swizzle,
  BuiltIn,
  Uniform,
  Vertex,
}

pub trait ShaderComponent {
  fn build_shader(&self, builder: &mut ShaderBuilder);
}

// impl ShaderComponent for Fog {
//   fn build_shader(&self, builder: &mut ShaderBuilder) {
//     let view_position = builder.query::<ViewPosition>();
//     let old_frag_color = builder.query::<FragColor>();
//     let fog = builder.fragment.uniform::<Self>();
//     builder.register::<FragColor>(fog.compute(view_position, old_frag_color))
//   }
// }

pub trait ShaderValue: Any {
  type ValueType;
  const NAME: &'static str = "unnamed";
}

pub enum ShaderValueStage {
  Vertex,
  Fragment,
}

pub struct ViewPosition;
impl ShaderValue for ViewPosition {
  type ValueType = Vec3<f32>;
}

pub struct WorldPosition;
impl ShaderValue for WorldPosition {
  type ValueType = Vec3<f32>;
}

pub struct ClipPosition;
impl ShaderValue for ClipPosition {
  type ValueType = Vec4<f32>;
}

pub struct FragColor;
impl ShaderValue for FragColor {
  type ValueType = Vec4<f32>;
}
