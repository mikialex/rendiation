use fast_hash_collection::FastHashSet;

use crate::*;

#[derive(Default)]
pub struct ForeignImplCollector {
  pub depend_user_functions: FastHashSet<String>,
  pub depend_user_struct: FastHashSet<String>,
}

impl ASTVisitor<FunctionCall> for ForeignImplCollector {
  fn visit(&mut self, ast: &FunctionCall) -> bool {
    if !ast.is_builtin() {
      self.depend_user_functions.insert(ast.name.name.clone());
    }
    true
  }
}

impl ASTVisitor<TypeExpression> for ForeignImplCollector {
  fn visit(&mut self, ast: &TypeExpression) -> bool {
    if let TypeExpression::Struct(s) = ast {
      self.depend_user_struct.insert(s.name.clone());
    }
    true
  }
}
