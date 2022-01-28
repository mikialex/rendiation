use std::collections::HashMap;

use crate::*;

pub struct CodeGenScopeCtx {
  ctx_guid: usize,
  var_guid: usize,
  pub code_gen_history: HashMap<ShaderGraphNodeRawHandleUntyped, MiddleVariableCodeGenResult>,
}

impl CodeGenScopeCtx {
  pub fn new(ctx_guid: usize) -> Self {
    Self {
      ctx_guid,
      var_guid: 0,
      code_gen_history: HashMap::new(),
    }
  }

  pub fn create_new_unique_name(&mut self) -> String {
    self.var_guid += 1;
    format!("v{}_{}", self.ctx_guid, self.var_guid)
  }
}

pub struct MiddleVariableCodeGenResult {
  pub var_name: String,
  pub statement: String,
}
