use arena_graph::*;

use lazy_static::lazy_static;
use std::{
  collections::HashSet,
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

pub struct ShaderGraph {
  pub attributes: HashSet<(ShaderGraphNodeHandleUntyped, usize)>,
  pub bindgroups_vertex: Vec<ShaderGraphBindGroup>,
  pub vertex_position: Option<ShaderGraphNodeHandleUntyped>,

  pub varyings: HashSet<(ShaderGraphNodeHandleUntyped, usize)>,
  pub bindgroups_frag: Vec<ShaderGraphBindGroup>,
  pub frag_outputs: HashSet<(ShaderGraphNodeHandleUntyped, usize)>,

  pub uniforms: HashSet<(ShaderGraphNodeHandleUntyped, usize)>,
  pub nodes: ArenaGraph<ShaderGraphNodeUntyped>,
}

impl ShaderGraph {
  fn new() -> Self {
    Self {
      uniforms: HashSet::new(),
      attributes: HashSet::new(),
      bindgroups_vertex: Vec::new(),
      bindgroups_frag: Vec::new(),
      nodes: ArenaGraph::new(),
      vertex_position: None,
      varyings: HashSet::new(),
      frag_outputs: HashSet::new(),
    }
  }
}

pub struct ShaderGraphBindGroup {
  inputs: Vec<ShaderGraphNodeHandleUntyped>,
}

impl ShaderGraphBindGroup {
  pub fn gen_header(&self, graph: &ShaderGraph) -> String {
    let result = String::new();
    result
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

  pub fn bindgroup(&mut self, b: impl FnOnce(&mut ShaderGraphBindGroupBuilder), is_vert: bool) {
    self.guard.as_mut().map(|g| {
      let mut builder = ShaderGraphBindGroupBuilder::new(g, is_vert);
      b(&mut builder);
      builder.resolve();
    });
  }
  pub fn bindgroup_vert(&mut self, b: impl FnOnce(&mut ShaderGraphBindGroupBuilder)) {
    self.bindgroup(b, true)
  }
  pub fn bindgroup_frag(&mut self, b: impl FnOnce(&mut ShaderGraphBindGroupBuilder)) {
    self.bindgroup(b, false)
  }
}

pub struct ShaderGraphBindGroupBuilder<'a> {
  index: usize,
  graph: &'a mut ShaderGraph,
  bindgroup: ShaderGraphBindGroup,
  is_vert: bool,
}

impl<'a> ShaderGraphBindGroupBuilder<'a> {
  pub fn new(graph: &'a mut ShaderGraph, is_vert: bool) -> Self {
    let index = if is_vert {
      graph.bindgroups_vertex.len()
    } else {
      graph.bindgroups_frag.len()
    };
    Self {
      index,
      graph,
      bindgroup: ShaderGraphBindGroup { inputs: Vec::new() },
      is_vert,
    }
  }

  pub fn uniform(&mut self, name: &str, node_type: NodeType) {
    let data = ShaderGraphNodeData::Input(ShaderGraphInputNode {
      node_type: ShaderGraphInputNodeType::Uniform,
      name: name.to_owned(),
    });
    let node = ShaderGraphNode::<AnyType>::new(data, node_type);
    let handle = self.graph.nodes.create_node(node);
    self.graph.uniforms.insert((handle, self.index));
    self.bindgroup.inputs.push(handle);
  }

  pub fn resolve(self) {
    if self.is_vert {
      self.graph.bindgroups_vertex.push(self.bindgroup)
    } else {
      self.graph.bindgroups_frag.push(self.bindgroup)
    }
  }
}

pub trait ShaderGraphDecorator {
  fn decorate(&self, graph: &mut ShaderGraph);
}
