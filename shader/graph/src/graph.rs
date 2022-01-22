use std::{any::TypeId, collections::HashMap};

use arena_graph::ArenaGraph;

use crate::{
  code_gen::{CodeBuilder, CodeGenCtx},
  Node, NodeUntyped, ShaderGraphNodeType, ShaderGraphNodeUntyped,
};

#[derive(Clone, Default)]
pub struct ShaderGraphInner {
  pub graph_guid: usize,
  pub nodes: ArenaGraph<ShaderGraphNodeUntyped>,
  pub parent: Option<Box<ShaderGraphInner>>,
}

pub struct ShaderGraphIncrementalBuilder {
  pub semantic_registered: HashMap<TypeId, NodeUntyped>,
  pub graph: ShaderGraphInner,
}

impl ShaderGraphIncrementalBuilder {
  pub fn insert_graph<T: ShaderGraphNodeType>(&self) -> Node<T> {
    todo!()
  }

  pub fn build(self) -> ShaderGraphBuilder {
    todo!()
  }
}

pub struct ShaderGraphBuilder {
  pub graph: ShaderGraphInner,
  pub code_gen: CodeGenCtx,
  pub code_builder: CodeBuilder,
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

impl ShaderGraphInner {
  pub fn build(self) -> ShaderGraphBuilder {
    todo!()
  }

  pub fn build_incremental(self) -> ShaderGraphIncrementalBuilder {
    todo!()
  }
}

pub fn create_shader_function() {
  let graph = ShaderGraphInner::default();

  //
}

// pub fn test_function(a: Node<f32>, b: Node<f32>) {
//   create_shader_function();
//   a + b
// }
