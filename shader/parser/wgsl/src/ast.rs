type Span = std::ops::Range<usize>;

use crate::lexer::{Lexer, Token};

#[derive(Clone, Copy, PartialEq)]
pub enum NumericType {
  Float,
  Int,
  UnsignedInt,
}

#[derive(Clone, Copy, PartialEq)]
pub enum PrimitiveType {
  Numeric(NumericType),
  Bool,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PrimitiveConstValue {
  Bool(bool),
  Numeric(NumericTypeConstValue),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum NumericTypeConstValue {
  Float(f32),
  Int(i32),
  UnsignedInt(u32),
}

#[derive(Debug)]
pub enum ParseError<'a> {
  Any(&'static str),
  Unexpected(Token<'a>, &'a str),
}

pub trait SyntaxElement: Sized {
  fn parse<'a>(input: &mut Lexer<'a>) -> Result<Self, ParseError<'a>>;
}

#[derive(Debug)]
pub struct FunctionDefine {
  pub name: Ident,
  pub arguments: Vec<(Ident, TypeExpression)>,
  pub return_type: Option<TypeExpression>,
  pub body: Block,
}

#[derive(Debug)]
pub struct Block {
  pub statements: Vec<Statement>,
}

#[derive(Debug)]
pub struct If {
  pub condition: Expression,
  pub accept: Block,
  pub elses: Vec<IfElse>,
  pub reject: Option<Block>,
}

#[derive(Debug)]
pub struct IfElse {
  pub condition: Expression,
  pub accept: Block,
}

#[derive(Debug)]
pub struct While {
  pub condition: Expression,
  pub body: Block,
}

#[derive(Debug)]
pub struct For {
  pub init: Box<Statement>,
  pub test: Box<Statement>,
  pub update: Expression,
  pub body: Block,
}

#[derive(Debug)]
pub enum Statement {
  Block(Block),
  Declare {
    ty: DeclarationType,
    name: Ident,
    init: Expression,
  },
  Empty,
  Expression(Expression),
  Return {
    value: Option<Expression>,
  },
  If(If),
  While(While),
  Break,
  Continue,
  For(For),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DeclarationType {
  Let,
  Const,
}

#[derive(Debug)]
pub enum TypeExpression {
  Named(Ident),
  // should we support this(generics) now?
  // Constructor {
  //     name: Ident,
  //     parameters: Vec<Box<TypeExpression>>,
  // },
}

#[derive(Debug)]
pub enum Expression {
  UnaryOperator {
    op: UnaryOperator,
    expr: Box<Self>,
  },
  BinaryOperator {
    left: Box<Self>,
    op: BinaryOperator,
    right: Box<Self>,
  },
  FunctionCall(FunctionCall),
  ArrayAccess {
    array: Box<Self>,
    index: Box<Self>,
  },
  ItemAccess {
    from: Box<Self>,
    to: Ident,
  },
  Assign {
    left: Ident,
    right: Box<Self>,
  },
  PrimitiveConst(PrimitiveConstValue),
  Ident(Ident),
}

#[derive(Debug)]
pub struct FunctionCall {
  pub name: String,
  pub arguments: Vec<Expression>,
}

#[derive(Debug)]
pub struct Ident {
  pub name: String,
}

#[derive(Copy, Clone, Debug)]
pub enum UnaryOperator {
  Neg,
  Not,
}

impl std::fmt::Display for UnaryOperator {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      UnaryOperator::Neg => write!(f, "-"),
      UnaryOperator::Not => write!(f, "!"),
    }
  }
}

#[derive(Copy, Clone, Debug)]
pub enum BinaryOperator {
  Add,
  Sub,
  Mul,
  Div,
  Mod,
  Less,
  LessEqual,
  Greater,
  GreaterEqual,
  Equal,
  NotEqual,
  And,
  Or,
  Xor,
  LogicalAnd,
  LogicalOr,
}

impl std::fmt::Display for BinaryOperator {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      BinaryOperator::Add => write!(f, "+"),
      BinaryOperator::Sub => write!(f, "-"),
      BinaryOperator::Mul => write!(f, "*"),
      BinaryOperator::Div => write!(f, "/"),
      BinaryOperator::Mod => write!(f, "%"),
      BinaryOperator::Less => write!(f, "<"),
      BinaryOperator::LessEqual => write!(f, "<="),
      BinaryOperator::Greater => write!(f, ">"),
      BinaryOperator::GreaterEqual => write!(f, ">="),
      BinaryOperator::Equal => write!(f, "=="),
      BinaryOperator::NotEqual => write!(f, "!="),
      BinaryOperator::And => write!(f, "&"),
      BinaryOperator::Or => write!(f, "|"),
      BinaryOperator::Xor => write!(f, "^"),
      BinaryOperator::LogicalAnd => write!(f, "&&"),
      BinaryOperator::LogicalOr => write!(f, "||"),
    }
  }
}
