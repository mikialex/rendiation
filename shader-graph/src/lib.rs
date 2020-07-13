use arena_graph::*;
use std::marker::PhantomData;

use lazy_static::lazy_static;
use std::sync::Mutex;

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

pub struct ShaderGraph {
  pub nodes: ArenaGraph<ShaderGraphNode<AnyType>>,
}

impl ShaderGraph {
  fn new() -> Self {
    Self {
      nodes: ArenaGraph::new(),
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
}

pub struct ShaderGraphNode<T> {
  phantom: PhantomData<T>,
  data: ShaderGraphNodeData,
  node_type: NodeType
}

impl<T> ShaderGraphNode<T> {
  pub fn new(data: ShaderGraphNodeData, node_type: NodeType) -> Self {
    Self {
      data,
      phantom: PhantomData,
      node_type
    }
  }
}

pub enum ShaderGraphNodeData {
  Function(FunctionNode),
  Uniform(UniformNode),
  Attribute(AttributeNode),
}

pub struct ShaderFunction{
  pub function_name: &'static str,
  pub function_source: &'static str,
}

pub struct FunctionNode {
  pub prototype: &'static ShaderFunction,
}

pub trait ShaderGraphDecorator {
  fn decorate(&self, graph: &mut ShaderGraph);
}
