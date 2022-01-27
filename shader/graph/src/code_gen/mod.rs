pub mod code_builder;
use std::collections::{HashMap, HashSet};

pub use code_builder::*;

pub mod shader;
pub use shader::*;

pub mod scope;
pub use scope::*;

pub mod targets;
pub use targets::*;

use crate::*;

impl ShaderGraphBuilder {
  pub fn get_node_gen_result_var<T>(&self, node: impl Into<Node<T>>) -> &str {
    let node = node.into().cast_untyped();
    self
      .scopes
      .iter()
      .rev()
      .find_map(|scope| scope.find_generated_node_exp(node))
      .unwrap()
  }

  fn add_fn_dep(&mut self, node: &FunctionNode) {
    self.depend_functions.insert(node.prototype);
  }

  fn gen_fn_depends(&self) -> String {
    let mut builder = CodeBuilder::default();
    let mut resolved_fn = HashSet::new();
    self.depend_functions.iter().for_each(|f| {
      if f.depend_functions.is_empty() {
        f.function_source.map(|s| builder.write_ln("").write_raw(s));
        resolved_fn.insert(f);
      }

      let mut fn_dep_graph = ArenaGraph::new();
      let mut resolving_fn = HashMap::new();
      let mut fn_to_expand = vec![f];

      while let Some(f) = fn_to_expand.pop() {
        let self_node_handle = *resolving_fn
          .entry(f)
          .or_insert_with(|| fn_dep_graph.create_node(f));
        f.depend_functions.iter().for_each(|f_d| {
          let dep_node_handle = resolving_fn
            .entry(f_d)
            .or_insert_with(|| fn_dep_graph.create_node(f_d));
          fn_dep_graph.connect_node(*dep_node_handle, self_node_handle);
        });
      }
      fn_dep_graph.traverse_dfs_in_topological_order(
        Handle::from_raw_parts(0, 0),
        &mut |n| {
          let f = n.data();
          if !resolved_fn.contains(f) {
            f.function_source.map(|s| builder.write_ln("").write_raw(s));
            resolved_fn.insert(f);
          }
        },
        &mut || panic!("loop exist"),
      )
    });
    builder.output()
  }
}
