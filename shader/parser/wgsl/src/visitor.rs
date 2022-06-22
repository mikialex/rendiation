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

struct Test;

impl ASTVisitor<PrimitiveType> for Test {
  fn visit(&mut self, ast: &PrimitiveType) -> bool {
    true
  }
}

pub trait WgslASTVisitor: ASTVisitor<PrimitiveType> + ASTVisitor<TextureType> {}

impl PrimitiveType {
  pub fn visit_by(&self, mut visitor: impl WgslASTVisitor) {
    if visitor.visit(self) {
      match self {
        PrimitiveType::Scalar(v) => todo!(),
        PrimitiveType::Vector(v) => todo!(),
        PrimitiveType::Texture(v) => v.visit_by(visitor),
        PrimitiveType::Sampler => {}
      }
    }
  }
}

impl TextureType {
  pub fn visit_by(&self, mut visitor: impl WgslASTVisitor) {
    visitor.visit(self);
  }
}
