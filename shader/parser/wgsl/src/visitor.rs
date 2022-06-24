use std::collections::HashSet;

use crate::*;

pub trait ASTVisitor<T> {
  fn visit(&mut self, ast: &T) -> bool {
    true
  }
}

impl<T, X> ASTVisitor<T> for X {
  default fn visit(&mut self, ast: &T) -> bool {
    true
  }
}

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

pub trait WgslASTVisitor: ASTVisitor<PrimitiveType> + ASTVisitor<TextureType> {}

impl<T> WgslASTVisitor for T where T: ASTVisitor<PrimitiveType> + ASTVisitor<TextureType> {}

pub trait ASTElement: Sized {
  fn visit_children<T>(&self, _visitor: &mut T) {
    // default don't have children
  }

  fn visit_by(&self, visitor: &mut impl ASTVisitor<Self>) {
    if visitor.visit(self) {
      self.visit_children(visitor)
    }
  }
}

struct Test;

impl ASTVisitor<PrimitiveType> for Test {
  fn visit(&mut self, ast: &PrimitiveType) -> bool {
    true
  }
}

#[test]
fn t() {
  let t: PrimitiveType = todo!();
  t.visit_by(&mut Test)
}

impl ASTElement for PrimitiveType {
  fn visit_children<T>(&self, visitor: &mut T) {
    match self {
      PrimitiveType::Scalar(v) => v.visit_by(visitor),
      PrimitiveType::Vector(v) => v.visit_by(visitor),
      PrimitiveType::Texture(v) => v.visit_by(visitor),
      PrimitiveType::Sampler => {}
    }
  }
}

impl ASTElement for TextureType {}
impl ASTElement for PrimitiveVectorType {}
impl ASTElement for PrimitiveValueType {}
