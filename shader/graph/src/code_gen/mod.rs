pub mod code_builder;
use std::collections::{HashMap, HashSet};

pub use code_builder::*;

pub mod shader;
pub use shader::*;

pub mod scope;
pub use scope::*;

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

impl ShaderGraphNodeData {
  fn gen_expr(&self, builder: &mut ShaderGraphBuilder) -> Option<String> {
    let expr = match self {
      ShaderGraphNodeData::Function(n) => {
        builder.add_fn_dep(n);
        format!(
          "{}({})",
          n.prototype.function_name,
          n.parameters
            .iter()
            .map(|from| { builder.get_node_gen_result_var(*from) })
            .collect::<Vec<_>>()
            .join(", ")
        )
      }
      ShaderGraphNodeData::BuiltInFunction { name, parameters } => todo!(),
      ShaderGraphNodeData::TextureSampling(n) => format!(
        "texture(sampler2D({}, {}), {})",
        builder.get_node_gen_result_var(n.texture),
        builder.get_node_gen_result_var(n.sampler),
        builder.get_node_gen_result_var(n.position),
      ),
      ShaderGraphNodeData::Swizzle { ty, source } => todo!(),
      ShaderGraphNodeData::Compose(_) => todo!(),
      ShaderGraphNodeData::Operator(_) => todo!(),
      ShaderGraphNodeData::Input(_) => todo!(),
      ShaderGraphNodeData::Output(_) => todo!(),
      ShaderGraphNodeData::Named(_) => todo!(),
      ShaderGraphNodeData::FieldGet {
        field_name,
        struct_node,
      } => todo!(),
      ShaderGraphNodeData::StructConstruct { struct_id, fields } => todo!(),
      ShaderGraphNodeData::Const(_) => todo!(),
      ShaderGraphNodeData::Scope(_) => todo!(),
    };
    expr.into()
  }
}
