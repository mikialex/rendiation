use std::{
  any::TypeId,
  collections::{HashMap, HashSet},
};

use arena_graph::ArenaGraph;

use crate::*;

pub struct ShaderGraphBuilder {
  scope_count: usize,
  pub scopes: Vec<ShaderGraphScopeBuilder>,
  pub depend_functions: HashSet<&'static ShaderFunctionMetaInfo>,
  pub struct_defines: HashMap<TypeId, &'static ShaderStructMetaInfo>,
}

impl Default for ShaderGraphBuilder {
  fn default() -> Self {
    Self {
      scope_count: 0,
      scopes: vec![ShaderGraphScopeBuilder::new(0)],
      depend_functions: Default::default(),
      struct_defines: Default::default(),
    }
  }
}

impl ShaderGraphBuilder {
  pub fn top_scope(&mut self) -> &mut ShaderGraphScopeBuilder {
    self.scopes.last_mut().unwrap()
  }

  pub fn push_scope(&mut self) -> &mut ShaderGraphScopeBuilder {
    self.scope_count += 1;
    self
      .scopes
      .push(ShaderGraphScopeBuilder::new(self.scope_count));
    self.top_scope()
  }

  pub fn pop_scope(&mut self) -> ShaderGraphScopeBuildResult {
    let top = self.scopes.pop().unwrap();
    ShaderGraphScopeBuildResult {
      code: top.code_builder.output(),
    }
  }

  pub fn compile(mut self) -> String {
    self.scopes.pop().unwrap().code_builder.output()
  }
}

pub struct ShaderGraphScopeBuilder {
  pub graph_guid: usize,
  pub code_gen: CodeGenScopeCtx,
  pub code_builder: CodeBuilder,
  pub nodes: ArenaGraph<ShaderGraphNodeUntyped>,
}

#[derive(Clone)]
pub struct ShaderGraphScopeBuildResult {
  pub code: String,
}

impl ShaderGraphScopeBuilder {
  pub fn new(graph_guid: usize) -> Self {
    Self {
      graph_guid,
      code_gen: CodeGenScopeCtx::new(graph_guid),
      code_builder: Default::default(),
      nodes: Default::default(),
    }
  }

  pub fn get_node_gen_result_var(&self, node: ShaderGraphNodeRawHandleUntyped) -> Option<&str> {
    if node.graph_id != node.graph_id {
      return None;
    }
    self
      .code_gen
      .code_gen_history
      .get(&node)
      .map(|v| v.var_name.as_ref())
  }

  pub fn insert_node<T: ShaderGraphNodeType>(&mut self, node: ShaderGraphNode<T>) -> NodeUntyped {
    ShaderGraphNodeRawHandle {
      handle: self.nodes.create_node(node.into_any()),
      graph_id: self.graph_guid,
    }
    .into()
  }

  pub fn build(self, parent: Option<Box<CodeGenScopeCtx>>) -> String {
    todo!()
  }
}
