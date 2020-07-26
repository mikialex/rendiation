use arena_graph::*;

use lazy_static::lazy_static;
use std::{
  collections::HashSet,
  sync::{Arc, Mutex},
};

mod code_builder;
mod code_gen;
pub mod nodes;
pub mod shader_function;
pub use nodes::*;
pub use shader_function::*;

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

pub trait ShaderGraphDecorator {
  fn decorate(&self, graph: &mut ShaderGraph);
}
