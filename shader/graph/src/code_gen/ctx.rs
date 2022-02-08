use std::collections::{HashMap, HashSet};

use crate::*;

pub struct CodeGenCtx {
  var_guid: usize,
  scopes: Vec<CodeGenScopeCtx>,
  depend_functions: HashSet<&'static ShaderFunctionMetaInfo>,
}

impl Default for CodeGenCtx {
  fn default() -> Self {
    Self {
      var_guid: Default::default(),
      scopes: vec![Default::default()],
      depend_functions: Default::default(),
    }
  }
}

impl CodeGenCtx {
  pub fn top_scope_mut(&mut self) -> &mut CodeGenScopeCtx {
    self.scopes.last_mut().unwrap()
  }
  pub fn top_scope(&self) -> &CodeGenScopeCtx {
    self.scopes.last().unwrap()
  }

  pub fn push_scope(&mut self) -> &mut CodeGenScopeCtx {
    self.scopes.push(Default::default());
    self.top_scope_mut()
  }

  pub fn pop_scope(&mut self) -> CodeGenScopeCtx {
    self.scopes.pop().unwrap()
  }

  pub fn add_fn_dep(&mut self, node: &FunctionNode) {
    self.depend_functions.insert(node.prototype);
  }

  pub fn gen_fn_depends(&self, builder: &mut CodeBuilder) {
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
  }

  pub fn try_get_node_gen_result_var<T>(&self, node: impl Into<Node<T>>) -> Option<&str> {
    let node = node.into().cast_untyped();
    self
      .scopes
      .iter()
      .rev()
      .find_map(|scope| scope.get_node_gen_result_var(node))
  }

  pub fn get_node_gen_result_var<T>(&self, node: impl Into<Node<T>>) -> &str {
    self.try_get_node_gen_result_var(node).unwrap()
  }

  pub fn create_new_unique_name(&mut self) -> String {
    self.var_guid += 1;
    format!("v{}", self.var_guid)
  }
}

#[derive(Default)]
pub struct CodeGenScopeCtx {
  pub code_gen_history: HashMap<ShaderGraphNodeRawHandleUntyped, MiddleVariableCodeGenResult>,
}

impl CodeGenScopeCtx {
  pub fn get_node_gen_result_var(&self, node: ShaderGraphNodeRawHandleUntyped) -> Option<&str> {
    self
      .code_gen_history
      .get(&node)
      .map(|v| v.var_name.as_ref())
  }
}

pub struct MiddleVariableCodeGenResult {
  pub var_name: String,
  pub statement: String,
}
