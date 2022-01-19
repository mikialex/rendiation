use crate::{Node, OperatorNode, ShaderGraphNodeData, ShaderGraphNodeType};
use std::ops::{Add, Div, Mul, Sub};

impl<T, U> Add for Node<T>
where
  U: ShaderGraphNodeType,
  T: ShaderGraphNodeType + Add<Output = U>,
{
  type Output = Node<U>;

  fn add(self, other: Self) -> Self::Output {
    ShaderGraphNodeData::Operator(OperatorNode {
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
    ShaderGraphNodeData::Operator(OperatorNode {
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
    ShaderGraphNodeData::Operator(OperatorNode {
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
    ShaderGraphNodeData::Operator(OperatorNode {
      left: self.cast_untyped(),
      right: other.cast_untyped(),
      operator: "/",
    })
    .insert_graph()
  }
}
