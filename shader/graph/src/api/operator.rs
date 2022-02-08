use crate::{Node, OperatorNode, ShaderGraphNodeExpr, ShaderGraphNodeType};
use std::ops::{Add, Div, Mul, Sub};

impl<T, U> Add for Node<T>
where
  U: ShaderGraphNodeType,
  T: ShaderGraphNodeType + Add<Output = U>,
{
  type Output = Node<U>;

  fn add(self, other: Self) -> Self::Output {
    ShaderGraphNodeExpr::Operator(OperatorNode {
      left: self.cast_untyped(),
      right: other.cast_untyped(),
      operator: "+",
    })
    .insert_graph()
  }
}

impl<T, U> Sub for Node<T>
where
  U: ShaderGraphNodeType,
  T: ShaderGraphNodeType + Sub<Output = U>,
{
  type Output = Node<U>;

  fn sub(self, other: Self) -> Self::Output {
    ShaderGraphNodeExpr::Operator(OperatorNode {
      left: self.cast_untyped(),
      right: other.cast_untyped(),
      operator: "-",
    })
    .insert_graph()
  }
}

impl<I, T, U> Mul<Node<I>> for Node<T>
where
  U: ShaderGraphNodeType,
  T: ShaderGraphNodeType,
  T: Mul<I, Output = U>,
{
  type Output = Node<U>;

  fn mul(self, other: Node<I>) -> Self::Output {
    ShaderGraphNodeExpr::Operator(OperatorNode {
      left: self.cast_untyped(),
      right: other.cast_untyped(),
      operator: "*",
    })
    .insert_graph()
  }
}

impl<I, T, U> Div<Node<I>> for Node<T>
where
  U: ShaderGraphNodeType,
  T: ShaderGraphNodeType,
  T: Div<I, Output = U>,
{
  type Output = Node<U>;

  fn div(self, other: Node<I>) -> Self::Output {
    ShaderGraphNodeExpr::Operator(OperatorNode {
      left: self.cast_untyped(),
      right: other.cast_untyped(),
      operator: "/",
    })
    .insert_graph()
  }
}

impl<T: PartialEq> Node<T> {
  pub fn equals(&self, other: Self) -> Node<bool> {
    ShaderGraphNodeExpr::Operator(OperatorNode {
      left: self.cast_untyped(),
      right: other.cast_untyped(),
      operator: "==",
    })
    .insert_graph()
  }

  pub fn not_equals(&self, other: Self) -> Node<bool> {
    ShaderGraphNodeExpr::Operator(OperatorNode {
      left: self.cast_untyped(),
      right: other.cast_untyped(),
      operator: "!=",
    })
    .insert_graph()
  }
}

impl<T: PartialOrd> Node<T> {
  pub fn less_than(&self, other: Self) -> Node<bool> {
    ShaderGraphNodeExpr::Operator(OperatorNode {
      left: self.cast_untyped(),
      right: other.cast_untyped(),
      operator: "<",
    })
    .insert_graph()
  }
  pub fn less_or_equal_than(&self, other: Self) -> Node<bool> {
    ShaderGraphNodeExpr::Operator(OperatorNode {
      left: self.cast_untyped(),
      right: other.cast_untyped(),
      operator: "<=",
    })
    .insert_graph()
  }
  pub fn greater_than(&self, other: impl Into<Self>) -> Node<bool> {
    ShaderGraphNodeExpr::Operator(OperatorNode {
      left: self.cast_untyped(),
      right: other.into().cast_untyped(),
      operator: ">",
    })
    .insert_graph()
  }
  pub fn greater_or_equal_than(&self, other: Self) -> Node<bool> {
    ShaderGraphNodeExpr::Operator(OperatorNode {
      left: self.cast_untyped(),
      right: other.cast_untyped(),
      operator: ">=",
    })
    .insert_graph()
  }
}

impl Node<bool> {
  #[must_use]
  pub fn or(&self, other: Self) -> Self {
    ShaderGraphNodeExpr::Operator(OperatorNode {
      left: self.cast_untyped(),
      right: other.cast_untyped(),
      operator: "||",
    })
    .insert_graph()
  }

  #[must_use]
  pub fn and(&self, other: Self) -> Self {
    ShaderGraphNodeExpr::Operator(OperatorNode {
      left: self.cast_untyped(),
      right: other.cast_untyped(),
      operator: "&&",
    })
    .insert_graph()
  }

  #[must_use]
  pub fn not(&self) -> Self {
    todo!()
    // ShaderGraphNodeExpr::Operator(OperatorNode {
    //   left: self.cast_untyped(),
    //   right: other.cast_untyped(),
    //   operator: "!",
    // })
    // .insert_graph()
  }
}
