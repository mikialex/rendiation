use crate::*;

impl<T, U> Add for Node<T>
where
  U: ShaderGraphNodeType,
  T: ShaderGraphNodeType + Add<Output = U>,
{
  type Output = Node<U>;

  fn add(self, other: Self) -> Self::Output {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.handle(),
      operator: BinaryOperator::Add,
    }
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
    OperatorNode::Binary {
      left: self.handle(),
      right: other.handle(),
      operator: BinaryOperator::Sub,
    }
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
    OperatorNode::Binary {
      left: self.handle(),
      right: other.handle(),
      operator: BinaryOperator::Mul,
    }
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
    OperatorNode::Binary {
      left: self.handle(),
      right: other.handle(),
      operator: BinaryOperator::Div,
    }
    .insert_graph()
  }
}

impl<T> Rem for Node<T>
where
  T: Rem<T, Output = T>,
  T: ShaderGraphNodeType,
{
  type Output = Node<T>;

  fn rem(self, rhs: Self) -> Self::Output {
    OperatorNode::Binary {
      left: self.handle(),
      right: rhs.handle(),
      operator: BinaryOperator::Rem,
    }
    .insert_graph()
  }
}

impl<T> AddAssign for Node<T>
where
  Self: Add<Output = Self> + Copy,
{
  fn add_assign(&mut self, rhs: Self) {
    *self = *self + rhs;
  }
}

impl<T> SubAssign for Node<T>
where
  Self: Sub<Output = Self> + Copy,
{
  fn sub_assign(&mut self, rhs: Self) {
    *self = *self - rhs;
  }
}

impl<T> MulAssign for Node<T>
where
  Self: Mul<Output = Self> + Copy,
{
  fn mul_assign(&mut self, rhs: Self) {
    *self = *self * rhs;
  }
}

impl<T> DivAssign for Node<T>
where
  Self: Div<Output = Self> + Copy,
{
  fn div_assign(&mut self, rhs: Self) {
    *self = *self / rhs;
  }
}

impl<T: PartialEq> Node<T> {
  pub fn equals(&self, other: Self) -> Node<bool> {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.handle(),
      operator: BinaryOperator::Eq,
    }
    .insert_graph()
  }

  pub fn not_equals(&self, other: Self) -> Node<bool> {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.handle(),
      operator: BinaryOperator::NotEq,
    }
    .insert_graph()
  }
}

impl<T: PartialOrd> Node<T> {
  pub fn less_than(&self, other: Self) -> Node<bool> {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.handle(),
      operator: BinaryOperator::LessThan,
    }
    .insert_graph()
  }
  pub fn less_or_equal_than(&self, other: Self) -> Node<bool> {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.handle(),
      operator: BinaryOperator::LessEqualThan,
    }
    .insert_graph()
  }
  pub fn greater_than(&self, other: impl Into<Self>) -> Node<bool> {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.into().handle(),
      operator: BinaryOperator::GreaterThan,
    }
    .insert_graph()
  }
  pub fn greater_or_equal_than(&self, other: Self) -> Node<bool> {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.handle(),
      operator: BinaryOperator::GreaterEqualThan,
    }
    .insert_graph()
  }
}

impl Node<bool> {
  #[must_use]
  pub fn or(&self, other: Self) -> Self {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.handle(),
      operator: BinaryOperator::LogicalOr,
    }
    .insert_graph()
  }

  #[must_use]
  pub fn and(&self, other: Self) -> Self {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.handle(),
      operator: BinaryOperator::LogicalAnd,
    }
    .insert_graph()
  }

  #[must_use]
  pub fn not(&self) -> Self {
    OperatorNode::Unary {
      operator: UnaryOperator::LogicalNot,
      one: self.handle(),
    }
    .insert_graph()
  }
}

impl<T, const U: usize> Node<Shader140Array<T, U>>
where
  T: ShaderGraphNodeType,
{
  pub fn index(&self, node: Node<impl ShaderGraphNodeType>) -> Node<T> {
    OperatorNode::Index {
      array: self.handle(),
      entry: node.handle(),
    }
    .insert_graph()
  }
}

impl<T, const U: usize> Node<[T; U]>
where
  T: ShaderGraphNodeType,
{
  pub fn index(&self, node: Node<impl ShaderGraphNodeType>) -> Node<T> {
    OperatorNode::Index {
      array: self.handle(),
      entry: node.handle(),
    }
    .insert_graph()
  }
}

impl<T, const U: usize> Node<BindingArray<T, U>>
where
  T: ShaderGraphNodeType,
{
  pub fn index(&self, node: Node<impl ShaderGraphNodeType>) -> Node<T> {
    OperatorNode::Index {
      array: self.handle(),
      entry: node.handle(),
    }
    .insert_graph()
  }
}
