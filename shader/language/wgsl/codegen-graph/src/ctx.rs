use fast_hash_collection::*;
use linked_hash_set::*;

use crate::*;

pub struct CodeGenCtx {
  var_guid: usize,
  scopes: Vec<CodeGenScopeCtx>,

  /// first generated binding structs(recursively)
  generated_binding_types: FastHashSet<&'static ShaderStructMetaInfo>,
  generated_unsized_binding_types: FastHashSet<&'static ShaderUnSizedStructMetaInfo>,

  /// new collected(recursively) in main function logic, deduplicate by self
  depend_functions: LinkedHashSet<&'static ShaderFunctionMetaInfo>,
  /// new collected(recursively) in main function logic, deduplicate by self and binding ones
  depend_types: LinkedHashSet<&'static ShaderStructMetaInfo>,

  uniform_array_wrappers: FastHashSet<ReWrappedPrimitiveArrayItem>,
}

impl Default for CodeGenCtx {
  fn default() -> Self {
    Self {
      var_guid: Default::default(),
      scopes: vec![Default::default()],
      generated_binding_types: Default::default(),
      generated_unsized_binding_types: Default::default(),
      depend_functions: Default::default(),
      depend_types: Default::default(),
      uniform_array_wrappers: Default::default(),
    }
  }
}

impl CodeGenCtx {
  pub fn top_scope_mut(&mut self) -> &mut CodeGenScopeCtx {
    self.scopes.last_mut().unwrap()
  }

  pub fn push_scope(&mut self) -> &mut CodeGenScopeCtx {
    self.scopes.push(Default::default());
    self.top_scope_mut()
  }

  pub fn pop_scope(&mut self) -> CodeGenScopeCtx {
    self.scopes.pop().unwrap()
  }

  /// note, recursive is done outside
  pub fn add_generated_binding_structs(&mut self, meta: &'static ShaderStructMetaInfo) -> bool {
    self.generated_binding_types.insert(meta)
  }
  pub fn add_generated_unsized_binding_structs(
    &mut self,
    meta: &'static ShaderUnSizedStructMetaInfo,
  ) -> bool {
    self.generated_unsized_binding_types.insert(meta)
  }

  pub fn add_special_uniform_array_wrapper(
    &mut self,
    wrapper: ReWrappedPrimitiveArrayItem,
  ) -> bool {
    self.uniform_array_wrappers.insert(wrapper)
  }

  pub fn add_fn_dep(&mut self, meta: &'static ShaderFunctionMetaInfo) {
    if self.depend_functions.insert_if_absent(meta) {
      for ty in meta.depend_types {
        self.add_struct_dep(ty)
      }
      for f in meta.depend_functions {
        self.add_fn_dep(f)
      }
    }
  }

  pub fn add_struct_dep(&mut self, meta: &'static ShaderStructMetaInfo) {
    if self.generated_binding_types.contains(meta) {
      return;
    }

    if self.depend_types.insert_if_absent(meta) {
      for f in meta.fields {
        self.add_ty_dep(f.ty);
      }
    }
  }

  fn add_ty_dep(&mut self, ty: ShaderStructMemberValueType) {
    match ty {
      ShaderStructMemberValueType::Primitive(_) => {}
      ShaderStructMemberValueType::Struct(s) => self.add_struct_dep(s),
      ShaderStructMemberValueType::FixedSizeArray((ty, _)) => self.add_ty_dep(*ty),
    }
  }

  pub fn gen_fn_and_ty_depends(
    &self,
    builder: &mut CodeBuilder,
    struct_gen: impl Fn(&mut CodeBuilder, &ShaderStructMetaInfoOwned),
  ) {
    for &ty in self.depend_types.iter().rev() {
      struct_gen(builder, &ty.to_owned())
    }

    for f in self.depend_functions.iter().rev() {
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
  pub code_gen_history: FastHashMap<ShaderGraphNodeRawHandle, MiddleVariableCodeGenResult>,
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
