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

impl<T, U> Mul for Node<T>
where
  U: ShaderGraphNodeType,
  T: ShaderGraphNodeType + Mul<Output = U>,
{
  type Output = Node<U>;

  fn mul(self, other: Self) -> Self::Output {
    ShaderGraphNodeData::Operator(OperatorNode {
      left: self.cast_untyped(),
      right: other.cast_untyped(),
      operator: "*",
    })
    .insert_graph()
  }
}

impl<T, U> Div for Node<T>
where
  U: ShaderGraphNodeType,
  T: ShaderGraphNodeType + Div<Output = U>,
{
  type Output = Node<U>;

  fn div(self, other: Self) -> Self::Output {
    ShaderGraphNodeData::Operator(OperatorNode {
      left: self.cast_untyped(),
      right: other.cast_untyped(),
      operator: "/",
    })
    .insert_graph()
  }
}
