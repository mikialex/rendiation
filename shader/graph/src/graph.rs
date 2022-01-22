use std::{any::TypeId, collections::HashMap};

use arena_graph::ArenaGraph;

use crate::{
  code_gen::{CodeBuilder, CodeGenCtx},
  Node, NodeUntyped, ShaderGraphNodeType, ShaderGraphNodeUntyped,
};

pub struct ShaderGraphBuilder {
  pub graph_guid: usize,
  pub code_gen: CodeGenCtx,
  pub code_builder: CodeBuilder,
  pub nodes: ArenaGraph<ShaderGraphNodeUntyped>,
  pub parent: Option<Box<Self>>,
}

#[derive(Clone)]
pub struct ShaderGraphScopeBuildResult{
  code: String
}

pub struct ShaderGraphIncrementalBuilder {
  pub semantic_registered: HashMap<TypeId, NodeUntyped>,
  pub graph: ShaderGraphBuilder,
}

impl ShaderGraphIncrementalBuilder {
  pub fn insert_graph<T: ShaderGraphNodeType>(&self) -> Node<T> {
    todo!()
  }

  pub fn build(self) -> ShaderGraphBuilder {
    todo!()
  }
}

impl ShaderGraphBuilder {
  pub fn new() -> Self {
    todo!()
  }

  pub fn insert_graph<T: ShaderGraphNodeType>(&self) -> Node<T> {
    todo!()
  }
  pub fn build(self, parent: Option<Box<CodeGenCtx>>) -> String {
    todo!()
  }
}

pub fn create_shader_function() {
  // let graph = ShaderGraphInner::default();

  //
}

// pub fn test_function(a: Node<f32>, b: Node<f32>) {
//   create_shader_function();
//   a + b
// }
