use std::{any::TypeId, collections::HashMap};

use arena_graph::ArenaGraph;

use crate::{
  code_gen::{CodeBuilder, CodeGenCtx},
  Node, NodeUntyped, ShaderGraphNode, ShaderGraphNodeData, ShaderGraphNodeType,
  ShaderGraphNodeUntyped,
};

pub struct ShaderGraphBuilder {
  pub graph_guid: usize,
  pub code_gen: CodeGenCtx,
  pub code_builder: CodeBuilder,
  pub nodes: ArenaGraph<ShaderGraphNodeUntyped>,
  pub type_id_map: HashMap<TypeId, &'static str>, // totally hack
  pub parent: Option<Box<Self>>,
}

#[derive(Clone)]
pub struct ShaderGraphScopeBuildResult {
  pub code: String,
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

  pub fn push_scope(mut self) -> Self {
    let mut scope = ShaderGraphBuilder::new();
    scope.parent = Box::new(self).into();
    scope
  }

  pub fn pop_scope(&mut self) -> ShaderGraphScopeBuildResult {
    todo!()
  }

  pub fn insert_node<T: ShaderGraphNodeType>(&mut self, node: ShaderGraphNode<T>) -> NodeUntyped {
    self.register_type::<T>();
    self.nodes.create_node(node.into_any()).into()
  }
  pub fn register_type<T: ShaderGraphNodeType>(&mut self) {
    self
      .type_id_map
      .entry(TypeId::of::<T>())
      .or_insert_with(T::to_glsl_type);
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
