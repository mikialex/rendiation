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

// struct Test;

// impl ASTVisitor<PrimitiveType> for Test {
//   fn visit(&mut self, ast: &PrimitiveType) -> bool {
//     true
//   }
// }

// #[test]
// fn t() {
//   let t: PrimitiveType = todo!();
//   t.visit_by(&mut Test)
// }

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
impl ASTElement for PrimitiveConstValue {}
impl ASTElement for TypeExpression {
  fn visit_children<T>(&self, visitor: &mut T) {
    match self {
      TypeExpression::Struct(i) => i.visit_by(visitor),
      TypeExpression::Primitive(p) => p.visit_by(visitor),
    }
  }
}

impl ASTElement for Ident {}
impl ASTElement for Expression {
  fn visit_children<T>(&self, visitor: &mut T) {
    match self {
      Expression::UnaryOperator { op, expr } => {
        op.visit_by(visitor);
        expr.visit_by(visitor);
      }
      Expression::BinaryOperator { left, op, right } => {
        left.visit_by(visitor);
        op.visit_by(visitor);
        right.visit_by(visitor);
      }
      Expression::FunctionCall(f) => f.visit_by(visitor),
      Expression::PrimitiveConstruct { ty, arguments } => {
        ty.visit_by(visitor);
        for arg in arguments {
          arg.visit_by(visitor)
        }
      }
      Expression::ArrayAccess { array, index } => {
        array.visit_by(visitor);
        index.visit_by(visitor);
      }
      Expression::ItemAccess { from, to } => {
        from.visit_by(visitor);
        to.visit_by(visitor);
      }
      Expression::PrimitiveConst(c) => c.visit_by(visitor),
      Expression::Ident(i) => i.visit_by(visitor),
    }
  }
}

impl ASTElement for BinaryOperator {}
impl ASTElement for UnaryOperator {}
impl ASTElement for FunctionCall {
  fn visit_children<T>(&self, visitor: &mut T) {
    self.name.visit_by(visitor);
    for arg in &self.arguments {
      arg.visit_by(visitor)
    }
  }
}

impl ASTElement for LhsExpression {
  fn visit_children<T>(&self, visitor: &mut T) {
    self.content.visit_by(visitor);

    for p in &self.postfix {
      p.visit_by(visitor);
    }
  }
}

impl ASTElement for LhsExpressionCore {
  fn visit_children<T>(&self, visitor: &mut T) {
    match self {
      LhsExpressionCore::Ident(i) => i.visit_by(visitor),
      LhsExpressionCore::Deref(d) => d.visit_by(visitor),
      LhsExpressionCore::Ref(d) => d.visit_by(visitor),
    }
  }
}

impl ASTElement for PostFixExpression {
  fn visit_children<T>(&self, visitor: &mut T) {
    match self {
      PostFixExpression::ArrayAccess { index } => index.visit_by(visitor),
      PostFixExpression::FieldAccess { field } => field.visit_by(visitor),
    }
  }
}

impl ASTElement for Block {
  fn visit_children<T>(&self, visitor: &mut T) {
    for s in &self.statements {
      s.visit_by(visitor)
    }
  }
}

impl ASTElement for Statement {
  fn visit_children<T>(&self, visitor: &mut T) {
    match self {
      Statement::Block(b) => b.visit_by(visitor),
      Statement::Declare {
        declare_ty,
        ty,
        name,
        init,
      } => {
        declare_ty.visit_by(visitor);
        if let Some(ty) = ty {
          ty.visit_by(visitor);
        }
        name.visit_by(visitor);

        if let Some(init) = init {
          init.visit_by(visitor);
        }
      }
      Statement::Empty => {}
      Statement::Assignment { lhs, value } => {
        lhs.visit_by(visitor);
        value.visit_by(visitor);
      }
      Statement::Expression(e) => e.visit_by(visitor),
      Statement::Return { value } => {
        if let Some(value) = value {
          value.visit_by(visitor)
        }
      }
      Statement::If(i) => i.visit_by(visitor),
      Statement::Switch(s) => s.visit_by(visitor),
      Statement::While(w) => w.visit_by(visitor),
      Statement::Loop { statements } => {
        for s in statements {
          s.visit_by(visitor)
        }
      }
      Statement::Break => {}
      Statement::Continue => {}
      Statement::Discard => {}
      Statement::For(f) => f.visit_by(visitor),
    }
  }
}
impl ASTElement for DeclarationType {}

impl ASTElement for For {
  fn visit_children<T>(&self, visitor: &mut T) {
    self.init.visit_by(visitor);
    self.test.visit_by(visitor);
    self.update.visit_by(visitor);
    self.body.visit_by(visitor);
  }
}

impl ASTElement for While {
  fn visit_children<T>(&self, visitor: &mut T) {
    self.condition.visit_by(visitor);
    self.body.visit_by(visitor);
  }
}

impl ASTElement for Switch {
  fn visit_children<T>(&self, visitor: &mut T) {
    self.target.visit_by(visitor);
    for case in &self.cases {
      case.visit_by(visitor);
    }
  }
}

impl ASTElement for CaseType {
  fn visit_children<T>(&self, visitor: &mut T) {
    if let CaseType::Const(v) = self {
      for e in v {
        e.visit_by(visitor)
      }
    }
  }
}

impl ASTElement for SwitchBody {
  fn visit_children<T>(&self, visitor: &mut T) {
    self.case.visit_by(visitor);
    for s in &self.statements {
      s.visit_by(visitor)
    }
  }
}

impl ASTElement for If {
  fn visit_children<T>(&self, visitor: &mut T) {
    self.condition.visit_by(visitor);
    self.accept.visit_by(visitor);
    for s in &self.elses {
      s.visit_by(visitor)
    }
    if let Some(re) = &self.reject {
      re.visit_by(visitor)
    }
  }
}

impl ASTElement for IfElse {
  fn visit_children<T>(&self, visitor: &mut T) {
    self.condition.visit_by(visitor);
    self.accept.visit_by(visitor);
  }
}
