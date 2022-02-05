use std::{
  any::TypeId,
  collections::{HashMap, HashSet},
};

use arena_graph::ArenaGraph;

use crate::*;

pub struct ShaderGraphBuilder {
  scope_count: usize,
  pub code_builder: CodeBuilder,
  pub scopes: Vec<ShaderGraphScopeBuilder>,
  pub depend_functions: HashSet<&'static ShaderFunctionMetaInfo>,
  pub struct_defines: HashMap<TypeId, &'static ShaderStructMetaInfo>,
}

impl Default for ShaderGraphBuilder {
  fn default() -> Self {
    let mut code_builder = CodeBuilder::default();
    code_builder.tab();
    Self {
      scope_count: 0,
      code_builder,
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
    self.code_builder.tab();
    self.top_scope()
  }

  pub fn pop_scope(&mut self) {
    self.scopes.pop().unwrap();
    self.code_builder.un_tab();
  }

  pub fn compile(self) -> String {
    self.code_builder.output()
  }
}

pub struct ShaderGraphScopeBuilder {
  pub graph_guid: usize,
  pub code_gen: CodeGenScopeCtx,
  pub nodes: ArenaGraph<ShaderGraphNodeUntyped>,
}

impl ShaderGraphScopeBuilder {
  pub fn new(graph_guid: usize) -> Self {
    Self {
      graph_guid,
      code_gen: CodeGenScopeCtx::new(graph_guid),
      nodes: Default::default(),
    }
  }

  pub fn get_node_gen_result_var(&self, node: ShaderGraphNodeRawHandleUntyped) -> Option<&str> {
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
}
