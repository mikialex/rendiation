use crate::*;

pub struct CodeGenCtx {
  var_guid: usize,
  scopes: Vec<CodeGenScopeCtx>,
  depend_functions: HashSet<&'static ShaderFunctionMetaInfo>,
  depend_types: HashSet<&'static ShaderStructMetaInfo>,
}

impl Default for CodeGenCtx {
  fn default() -> Self {
    Self {
      var_guid: Default::default(),
      scopes: vec![Default::default()],
      depend_functions: Default::default(),
      depend_types: Default::default(),
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

  pub fn add_fn_dep(&mut self, meta: &'static ShaderFunctionMetaInfo) {
    if self.depend_functions.insert(meta) {
      for ty in meta.depend_types {
        self.add_ty_dep(ty)
      }
      for f in meta.depend_functions {
        self.add_fn_dep(f)
      }
    }
  }

  fn add_ty_dep(&mut self, meta: &'static ShaderStructMetaInfo) {
    if self.depend_types.insert(meta) {
      for f in meta.fields {
        if let ShaderStructMemberValueType::Struct(s) = f.ty {
          self.add_ty_dep(s)
        }
      }
    }
  }

  pub fn gen_fn_and_ty_depends(
    &self,
    builder: &mut CodeBuilder,
    struct_gen: impl Fn(&mut CodeBuilder, &ShaderStructMetaInfoOwned),
  ) {
    for &ty in &self.depend_types {
      struct_gen(builder, &ty.to_owned())
    }

    for f in &self.depend_functions {
      builder.write_ln("").write_raw(f.function_source);
    }
  }

  pub fn try_get_node_gen_result_var(&self, node: ShaderGraphNodeRawHandle) -> Option<&str> {
    self
      .scopes
      .iter()
      .rev()
      .find_map(|scope| scope.get_node_gen_result_var(node))
  }

  pub fn get_node_gen_result_var(&self, node: ShaderGraphNodeRawHandle) -> &str {
    self.try_get_node_gen_result_var(node).unwrap()
  }

  pub fn create_new_unique_name(&mut self) -> String {
    self.var_guid += 1;
    format!("v{}", self.var_guid)
  }
}

#[derive(Default)]
pub struct CodeGenScopeCtx {
  pub code_gen_history: HashMap<ShaderGraphNodeRawHandle, MiddleVariableCodeGenResult>,
}

impl CodeGenScopeCtx {
  pub fn get_node_gen_result_var(&self, node: ShaderGraphNodeRawHandle) -> Option<&str> {
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
