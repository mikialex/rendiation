use arena_graph::*;

use lazy_static::lazy_static;
use std::{
  collections::{HashMap, HashSet},
  sync::{Arc, Mutex, MutexGuard},
};

mod code_builder;
mod code_gen;
pub mod nodes;
pub mod shader_function;
pub use nodes::*;
pub use shader_function::*;

lazy_static! {
  pub static ref IN_BUILDING_SHADER_GRAPH: Mutex<Option<ShaderGraph>> = Mutex::new(None);
}

pub struct AnyType {}
pub type ShaderGraphNodeHandle<T> = ArenaGraphNodeHandle<ShaderGraphNode<T>>;
pub type ShaderGraphNodeHandleUntyped = ShaderGraphNodeHandle<AnyType>;
pub type ShaderGraphNodeUntyped = ShaderGraphNode<AnyType>;

pub struct BindGroupInfo {}

pub struct ShaderGraph {
  pub uniforms_vertex: HashMap<String, ShaderGraphNodeHandleUntyped>,
  pub uniforms_frag: HashMap<String, ShaderGraphNodeHandleUntyped>,
  pub attributes: HashSet<ShaderGraphNodeHandleUntyped>,
  pub bindgroups: Vec<BindGroupInfo>,
  pub nodes: ArenaGraph<ShaderGraphNodeUntyped>,
  pub vertex_position: Option<ShaderGraphNodeHandleUntyped>,
  pub varyings: HashSet<ShaderGraphNodeHandleUntyped>,
  pub frag_outputs: HashSet<ShaderGraphNodeHandleUntyped>,
}

impl ShaderGraph {
  fn new() -> Self {
    Self {
      uniforms_vertex: HashMap::new(),
      uniforms_frag: HashMap::new(),
      attributes: HashSet::new(),
      bindgroups: Vec::new(),
      nodes: ArenaGraph::new(),
      vertex_position: None,
      varyings: HashSet::new(),
      frag_outputs: HashSet::new(),
    }
  }
}

/// The builder will hold the mutex guard to make sure the in building shadergraph is singleton
pub struct ShaderGraphBuilder<'a> {
  guard: MutexGuard<'a, Option<ShaderGraph>>,
}

impl<'a> ShaderGraphBuilder<'a> {
  pub fn new() -> Self {
    let mut guard = IN_BUILDING_SHADER_GRAPH.lock().unwrap();
    *guard = Some(ShaderGraph::new());

    Self { guard }
  }

  pub fn create(mut self) -> ShaderGraph {
    self.guard.take().unwrap()
  }

  pub fn uniform(&mut self, name: &str, ty: NodeType) {}
}

pub trait ShaderGraphDecorator {
  fn decorate(&self, graph: &mut ShaderGraph);
}
