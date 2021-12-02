use std::marker::PhantomData;

use arena_graph::{ArenaGraph, ArenaGraphNodeHandle};
use rendiation_algebra::*;

pub struct ShaderBuilder {
  vertex: ShaderGraph,
  fragment: ShaderGraph,
}

impl ShaderBuilder {
  pub fn query<T: ShaderValue>(&self) -> Node<T::ValueType> {
    todo!()
  }

  pub fn register<T: ShaderValue>(&mut self, node: Node<T::ValueType>) {
    todo!()
  }
}

pub trait ShaderValue {
  type ValueType;
}

pub struct ViewPosition;
impl ShaderValue for ViewPosition {
  type ValueType = Vec3<f32>;
}
pub struct FragColor;
impl ShaderValue for FragColor {
  type ValueType = Vec4<f32>;
}

pub struct ShaderGraph {
  graph: ArenaGraph<NodeData>,
  uniforms: Vec<NodeUnTyped>,
  input: Vec<NodeUnTyped>,
  output: Vec<NodeUnTyped>,
}

pub struct NodeUnTyped {}

pub struct Node<T> {
  ty: PhantomData<T>,
  handle: ArenaGraphNodeHandle<NodeData>,
}

pub enum NodeData {
  FunctionCall,
  Uniform,
}

pub trait ShaderComponent {
  fn build_shader(&self, builder: &mut ShaderBuilder);
}

impl ShaderComponent for Fog {
  fn build_shader(&self, builder: &mut ShaderBuilder) {
    let view_position = builder.query::<ViewPosition>();
    let old_frag_color = builder.query::<FragColor>();
    let fog = builder.fragment.uniform::<Self>();
    builder.register::<FragColor>(fog.compute(view_position, old_frag_color))
  }
}
