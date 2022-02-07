use std::{
  any::TypeId,
  collections::{HashMap, HashSet},
};

use arena_graph::ArenaGraph;

use crate::*;

pub struct ShaderGraphBuilder {
  scope_count: usize,
  pub scopes: Vec<ShaderGraphScope>,
  pub struct_defines: HashMap<TypeId, &'static ShaderStructMetaInfo>,
}

impl Default for ShaderGraphBuilder {
  fn default() -> Self {
    Self {
      scope_count: 0,
      scopes: vec![ShaderGraphScope::new(0)],
      struct_defines: Default::default(),
    }
  }
}

impl ShaderGraphBuilder {
  pub fn top_scope_mut(&mut self) -> &mut ShaderGraphScope {
    self.scopes.last_mut().unwrap()
  }
  pub fn top_scope(&self) -> &ShaderGraphScope {
    self.scopes.last().unwrap()
  }

  pub fn push_scope(&mut self) -> &mut ShaderGraphScope {
    self.scope_count += 1;
    self.scopes.push(ShaderGraphScope::new(self.scope_count));
    self.top_scope_mut()
  }

  pub fn pop_scope(&mut self) -> ShaderGraphScope {
    self.scopes.pop().unwrap()
  }
}

pub struct ShaderGraphScope {
  pub graph_guid: usize,
  pub has_side_effect: bool,
  pub nodes: ArenaGraph<ShaderGraphNodeUntyped>,
  pub inserted: Vec<ShaderGraphNodeRawHandleUntyped>,
  pub barriers: Vec<ShaderGraphNodeRawHandleUntyped>,
  pub captured: Vec<ShaderGraphNodeRawHandleUntyped>,
}

impl ShaderGraphScope {
  pub fn new(graph_guid: usize) -> Self {
    Self {
      graph_guid,
      has_side_effect: false,
      nodes: Default::default(),
      inserted: Default::default(),
      barriers: Default::default(),
      captured: Default::default(),
    }
  }

  pub fn insert_node<T: ShaderGraphNodeType>(&mut self, node: ShaderGraphNode<T>) -> NodeUntyped {
    let handle = ShaderGraphNodeRawHandle {
      handle: self.nodes.create_node(node.into_any()),
      graph_id: self.graph_guid,
    };
    self.inserted.push(handle);
    handle.into()
  }
}
