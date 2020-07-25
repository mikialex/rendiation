use arena_graph::*;
use std::marker::PhantomData;

use lazy_static::lazy_static;
use std::{
  collections::HashSet,
  hash::{Hash, Hasher},
  sync::{Arc, Mutex},
};

mod code_builder;
mod code_gen;
pub mod nodes;
pub use nodes::*;

lazy_static! {
  // we should remove mutex and use unsafe in future
  pub static ref IN_BUILDING_SHADER_GRAPH: Mutex<Option<ShaderGraph>> = Mutex::new(None);
}

pub struct AnyType {}
pub type ShaderGraphNodeHandle<T> = ArenaGraphNodeHandle<ShaderGraphNode<T>>;
pub type ShaderGraphNodeHandleUntyped = ShaderGraphNodeHandle<AnyType>;
pub type ShaderGraphNodeUntyped = ShaderGraphNode<AnyType>;

pub struct ShaderGraph {
  pub uniforms: HashSet<ShaderGraphNodeHandleUntyped>,
  pub attributes: HashSet<ShaderGraphNodeHandleUntyped>,
  pub nodes: ArenaGraph<ShaderGraphNodeUntyped>,
  pub vertex_position: Option<ShaderGraphNodeHandleUntyped>,
  pub varyings: HashSet<ShaderGraphNodeHandleUntyped>,
  pub frag_outputs: HashSet<ShaderGraphNodeHandleUntyped>,
}

impl ShaderGraph {
  fn new() -> Self {
    Self {
      uniforms: HashSet::new(),
      attributes: HashSet::new(),
      nodes: ArenaGraph::new(),
      vertex_position: None,
      varyings: HashSet::new(),
      frag_outputs: HashSet::new(),
    }
  }
}

pub struct ShaderGraphBuilder {}

impl ShaderGraphBuilder {
  pub fn new() -> Self {
    let mut guard = IN_BUILDING_SHADER_GRAPH.lock().unwrap();
    let graph = guard.as_mut();
    if graph.is_some() {
      panic!("already has one graph in building")
    }

    *guard = Some(ShaderGraph::new());

    Self {}
  }

  pub fn create(self) -> ShaderGraph {
    IN_BUILDING_SHADER_GRAPH.lock().unwrap().take().unwrap()
  }

  // pub fn uniform(&mut self, name: &str,  ) {
  // }
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
  name: String,
}

pub enum ShaderGraphInputNodeType {
  Uniform,
  Attribute,
}

#[derive(Debug, Eq)]
pub struct ShaderFunction {
  pub function_name: &'static str,
  pub function_source: &'static str,
  pub depend_functions: HashSet<Arc<ShaderFunction>>,
}

impl ShaderFunction {
  pub fn declare_function_dep(mut self, f: Arc<ShaderFunction>) -> Self {
    self.depend_functions.insert(f);
    self
  }
}

impl Hash for ShaderFunction {
  fn hash<H>(&self, state: &mut H)
  where
    H: Hasher,
  {
    self.function_name.hash(state);
  }
}

impl PartialEq for ShaderFunction {
  fn eq(&self, other: &Self) -> bool {
    self.function_name == other.function_name
  }
}

impl ShaderFunction {
  pub fn new(function_name: &'static str, function_source: &'static str) -> Self {
    Self {
      function_name,
      function_source,
      depend_functions: HashSet::new(),
    }
  }
}

pub struct FunctionNode {
  pub prototype: Arc<ShaderFunction>,
}

pub trait ShaderGraphDecorator {
  fn decorate(&self, graph: &mut ShaderGraph);
}
