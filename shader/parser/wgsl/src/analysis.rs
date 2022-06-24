use crate::*;
use std::collections::HashSet;

struct ForeignImplCollector {
  depend_user_functions: HashSet<String>,
  depend_user_struct: HashSet<String>,
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
