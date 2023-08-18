use crate::*;

pub enum UnaryOperator {
  LogicalNot,
  Neg,
}

pub enum BinaryOperator {
  Add,
  Sub,
  Mul,
  Div,
  Rem,
  Eq,
  NotEq,
  GreaterThan,
  LessThan,
  GreaterEqualThan,
  LessEqualThan,
  LogicalOr,
  LogicalAnd,
  BitAnd,
  BitOr,
}
pub enum OperatorNode {
  Unary {
    one: ShaderNodeRawHandle,
    operator: UnaryOperator,
  },
  Binary {
    left: ShaderNodeRawHandle,
    right: ShaderNodeRawHandle,
    operator: BinaryOperator,
  },
  Index {
    array: ShaderNodeRawHandle,
    entry: ShaderNodeRawHandle,
  },
  IndexStatic {
    array: ShaderNodeRawHandle,
    entry: u32,
  },
}

impl OperatorNode {
  pub fn insert_api<T: ShaderNodeType>(self) -> Node<T> {
    ShaderNodeExpr::Operator(self).insert_api()
  }
}

impl<T, U> Add for Node<T>
where
  U: ShaderNodeType,
  T: ShaderNodeType + Add<Output = U>,
{
  type Output = Node<U>;

  fn add(self, other: Self) -> Self::Output {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.handle(),
      operator: BinaryOperator::Add,
    }
    .insert_api()
  }
}

impl<T, U> Sub for Node<T>
where
  U: ShaderNodeType,
  T: ShaderNodeType + Sub<Output = U>,
{
  type Output = Node<U>;

  fn sub(self, other: Self) -> Self::Output {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.handle(),
      operator: BinaryOperator::Sub,
    }
    .insert_api()
  }
}

impl<I, T, U> Mul<Node<I>> for Node<T>
where
  U: ShaderNodeType,
  T: ShaderNodeType,
  T: Mul<I, Output = U>,
{
  type Output = Node<U>;

  fn mul(self, other: Node<I>) -> Self::Output {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.handle(),
      operator: BinaryOperator::Mul,
    }
    .insert_api()
  }
}

impl<T, U> Div for Node<T>
where
  U: ShaderNodeType,
  T: ShaderNodeType,
  T: Div<Output = U>,
{
  type Output = Node<U>;

  fn div(self, other: Self) -> Self::Output {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.handle(),
      operator: BinaryOperator::Div,
    }
    .insert_api()
  }
}

impl<T> Rem for Node<T>
where
  T: Rem<T, Output = T>,
  T: ShaderNodeType,
{
  type Output = Node<T>;

  fn rem(self, rhs: Self) -> Self::Output {
    OperatorNode::Binary {
      left: self.handle(),
      right: rhs.handle(),
      operator: BinaryOperator::Rem,
    }
    .insert_api()
  }
}

impl<T> BitAnd for Node<T>
where
  T: BitAnd<T, Output = T>,
  T: ShaderNodeType,
{
  type Output = Node<T>;

  fn bitand(self, rhs: Self) -> Self::Output {
    OperatorNode::Binary {
      left: self.handle(),
      right: rhs.handle(),
      operator: BinaryOperator::BitAnd,
    }
    .insert_api()
  }
}

impl<T> BitOr for Node<T>
where
  T: BitOr<T, Output = T>,
  T: ShaderNodeType,
{
  type Output = Node<T>;

  fn bitor(self, rhs: Self) -> Self::Output {
    OperatorNode::Binary {
      left: self.handle(),
      right: rhs.handle(),
      operator: BinaryOperator::BitOr,
    }
    .insert_api()
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

/// todo restrict
impl<T: ShaderNodeType> Neg for Node<T> {
  type Output = Self;
  fn neg(self) -> Self::Output {
    OperatorNode::Unary {
      one: self.handle(),
      operator: UnaryOperator::Neg,
    }
    .insert_api()
  }
}

impl<T: PartialEq> Node<T> {
  pub fn equals(&self, other: impl Into<Self>) -> Node<bool> {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.into().handle(),
      operator: BinaryOperator::Eq,
    }
    .insert_api()
  }

  pub fn not_equals(&self, other: impl Into<Self>) -> Node<bool> {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.into().handle(),
      operator: BinaryOperator::NotEq,
    }
    .insert_api()
  }
}

impl<T: PartialOrd> Node<T> {
  pub fn less_than(&self, other: impl Into<Self>) -> Node<bool> {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.into().handle(),
      operator: BinaryOperator::LessThan,
    }
    .insert_api()
  }
  pub fn less_equal_than(&self, other: impl Into<Self>) -> Node<bool> {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.into().handle(),
      operator: BinaryOperator::LessEqualThan,
    }
    .insert_api()
  }
  pub fn greater_than(&self, other: impl Into<Self>) -> Node<bool> {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.into().handle(),
      operator: BinaryOperator::GreaterThan,
    }
    .insert_api()
  }
  pub fn greater_equal_than(&self, other: impl Into<Self>) -> Node<bool> {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.into().handle(),
      operator: BinaryOperator::GreaterEqualThan,
    }
    .insert_api()
  }
}

impl Node<bool> {
  #[must_use]
  pub fn or(&self, other: impl Into<Self>) -> Self {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.into().handle(),
      operator: BinaryOperator::LogicalOr,
    }
    .insert_api()
  }

  #[must_use]
  pub fn and(&self, other: impl Into<Self>) -> Self {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.into().handle(),
      operator: BinaryOperator::LogicalAnd,
    }
    .insert_api()
  }

  #[must_use]
  pub fn not(&self) -> Self {
    OperatorNode::Unary {
      operator: UnaryOperator::LogicalNot,
      one: self.handle(),
    }
    .insert_api()
  }
}

// impl<T, const U: usize> Node<Shader140Array<T, U>>
// where
//   T: ShaderNodeType,
// {
//   pub fn index(&self, node: Node<impl ShaderNodeType>) -> Node<T> {
//     OperatorNode::Index {
//       array: self.handle(),
//       entry: node.handle(),
//     }
//     .insert_api()
//   }
// }
impl<T, const U: usize> UniformNode<Shader140Array<T, U>>
where
  T: ShaderNodeType,
{
  pub fn index(&self, node: Node<impl ShaderNodeType>) -> UniformNode<T> {
    OperatorNode::Index {
      array: self.handle(),
      entry: node.handle(),
    }
    .insert_api()
  }
}

// impl<T, const U: usize> Node<[T; U]>
// where
//   T: ShaderNodeType,
// {
//   pub fn index(&self, node: Node<impl ShaderNodeType>) -> Node<T> {
//     OperatorNode::Index {
//       array: self.handle(),
//       entry: node.handle(),
//     }
//     .insert_api()
//   }
// }
impl<T, const U: usize, const S: AddressSpace> Node<ShaderPtr<[T; U], S>>
where
  T: ShaderNodeType,
{
  pub fn index(&self, node: Node<impl ShaderNodeType>) -> Node<ShaderPtr<T, S>> {
    OperatorNode::Index {
      array: self.handle(),
      entry: node.handle(),
    }
    .insert_api()
  }
}

impl<T, const U: usize, const S: AddressSpace> Node<ShaderPtr<BindingArray<T, U>, S>>
where
  T: ShaderNodeType,
{
  pub fn index(&self, node: Node<impl ShaderNodeType>) -> Node<ShaderPtr<T, S>> {
    OperatorNode::Index {
      array: self.handle(),
      entry: node.handle(),
    }
    .insert_api()
  }
}
