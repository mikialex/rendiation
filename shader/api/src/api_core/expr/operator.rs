use crate::*;

pub enum UnaryOperator {
  LogicalNot,
  BitwiseNot,
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
  BitXor,
  ShiftLeft,
  /// Right shift carries the sign of signed integers only.
  ShiftRight,
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
}

/// # Safety
///
/// the field index should be bounded
///
/// .
pub unsafe fn index_access_field(
  struct_node: ShaderNodeRawHandle,
  field_index: usize,
) -> ShaderNodeRawHandle {
  ShaderNodeExpr::IndexStatic {
    field_index,
    target: struct_node,
  }
  .insert_api_raw()
}

/// # Safety
///
/// the field index should be bounded, and it is a pointer type
///
/// .
pub unsafe fn index_access_field_as_ptr(
  struct_node: ShaderNodeRawHandle,
  field_index: usize,
) -> BoxedShaderPtr {
  unsafe { Box::new(index_access_field(struct_node, field_index)) }
}

impl OperatorNode {
  pub fn insert_api_raw(self) -> ShaderNodeRawHandle {
    ShaderNodeExpr::Operator(self).insert_api_raw()
  }
  pub fn insert_api<T: ShaderNodeType>(self) -> Node<T> {
    ShaderNodeExpr::Operator(self).insert_api()
  }
}

/// The operand types of the component-wise arithmetic operators(`+ - * / %` with same type
/// operands), which are the numeric scalars and the numeric vectors.
///
/// see <https://www.w3.org/TR/WGSL/#arithmetic-expr>
pub trait ShaderComponentWiseArithmeticType: ShaderNodeType {}

/// The operand types of the `+` and `-` operators, which additionally include the float matrices.
pub trait ShaderAddSubType: ShaderNodeType {}

/// All the valid multiplications in WGSL: the component-wise multiplication, the scalar and vector
/// (or matrix) mixed multiplication, and the linear algebra matrix multiplication.
///
/// The math library's operator implementations are not reused, because they contain operations
/// that are invalid in WGSL, for example the homogeneous `Mat4 * Vec3`.
pub trait ShaderMul<Rhs>: ShaderNodeType {
  type Output: ShaderNodeType;
}

macro_rules! impl_numeric_arithmetic {
  ($($ty: ty),+) => {
    $(
      impl ShaderComponentWiseArithmeticType for $ty {}
      impl ShaderAddSubType for $ty {}
      impl ShaderMul<$ty> for $ty {
        type Output = $ty;
      }
    )+
  };
}

macro_rules! impl_shader_mul {
  ($lhs: ty, $rhs: ty, $output: ty) => {
    impl ShaderMul<$rhs> for $lhs {
      type Output = $output;
    }
  };
}

macro_rules! impl_scalar_vector_arithmetic {
  ($scalar: ty) => {
    impl_numeric_arithmetic!($scalar, Vec2<$scalar>, Vec3<$scalar>, Vec4<$scalar>);
    impl_shader_mul!(Vec2<$scalar>, $scalar, Vec2<$scalar>);
    impl_shader_mul!(Vec3<$scalar>, $scalar, Vec3<$scalar>);
    impl_shader_mul!(Vec4<$scalar>, $scalar, Vec4<$scalar>);
    impl_shader_mul!($scalar, Vec2<$scalar>, Vec2<$scalar>);
    impl_shader_mul!($scalar, Vec3<$scalar>, Vec3<$scalar>);
    impl_shader_mul!($scalar, Vec4<$scalar>, Vec4<$scalar>);
  };
}

impl_scalar_vector_arithmetic!(f32);
impl_scalar_vector_arithmetic!(u32);
impl_scalar_vector_arithmetic!(i32);

macro_rules! impl_matrix_scalar_arithmetic {
  ($($mat: ty),+) => {
    $(
      impl ShaderAddSubType for $mat {}
      impl_shader_mul!($mat, f32, $mat);
      impl_shader_mul!(f32, $mat, $mat);
    )+
  };
}

impl_matrix_scalar_arithmetic!(Mat2<f32>, Mat3<f32>, Mat4<f32>, Mat4x3<f32>);

// matCxR * vecC -> vecR
impl_shader_mul!(Mat2<f32>, Vec2<f32>, Vec2<f32>);
impl_shader_mul!(Mat3<f32>, Vec3<f32>, Vec3<f32>);
impl_shader_mul!(Mat4<f32>, Vec4<f32>, Vec4<f32>);
impl_shader_mul!(Mat4x3<f32>, Vec4<f32>, Vec3<f32>);

// vecR * matCxR -> vecC
impl_shader_mul!(Vec2<f32>, Mat2<f32>, Vec2<f32>);
impl_shader_mul!(Vec3<f32>, Mat3<f32>, Vec3<f32>);
impl_shader_mul!(Vec4<f32>, Mat4<f32>, Vec4<f32>);
impl_shader_mul!(Vec3<f32>, Mat4x3<f32>, Vec4<f32>);

// matKxR * matCxK -> matCxR
impl_shader_mul!(Mat2<f32>, Mat2<f32>, Mat2<f32>);
impl_shader_mul!(Mat3<f32>, Mat3<f32>, Mat3<f32>);
impl_shader_mul!(Mat4<f32>, Mat4<f32>, Mat4<f32>);
impl_shader_mul!(Mat4x3<f32>, Mat4<f32>, Mat4x3<f32>);
impl_shader_mul!(Mat3<f32>, Mat4x3<f32>, Mat4x3<f32>);

impl<T: ShaderAddSubType> Add for Node<T> {
  type Output = Self;

  fn add(self, other: Self) -> Self::Output {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.handle(),
      operator: BinaryOperator::Add,
    }
    .insert_api()
  }
}

impl<T: ShaderAddSubType> Sub for Node<T> {
  type Output = Self;

  fn sub(self, other: Self) -> Self::Output {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.handle(),
      operator: BinaryOperator::Sub,
    }
    .insert_api()
  }
}

impl<I, T> Mul<Node<I>> for Node<T>
where
  T: ShaderMul<I>,
{
  type Output = Node<T::Output>;

  fn mul(self, other: Node<I>) -> Self::Output {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.handle(),
      operator: BinaryOperator::Mul,
    }
    .insert_api()
  }
}

impl<T: ShaderComponentWiseArithmeticType> Div for Node<T> {
  type Output = Self;

  fn div(self, other: Self) -> Self::Output {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.handle(),
      operator: BinaryOperator::Div,
    }
    .insert_api()
  }
}

impl<T: ShaderComponentWiseArithmeticType> Rem for Node<T> {
  type Output = Self;

  fn rem(self, rhs: Self) -> Self::Output {
    OperatorNode::Binary {
      left: self.handle(),
      right: rhs.handle(),
      operator: BinaryOperator::Rem,
    }
    .insert_api()
  }
}

impl<T> Shl<Node<T::Shape<u32>>> for Node<T>
where
  T: ShaderScalarOrVec,
  T::Item: ShaderIntType,
{
  type Output = Self;

  fn shl(self, rhs: Node<T::Shape<u32>>) -> Self::Output {
    OperatorNode::Binary {
      left: self.handle(),
      right: rhs.handle(),
      operator: BinaryOperator::ShiftLeft,
    }
    .insert_api()
  }
}

impl<T> Shr<Node<T::Shape<u32>>> for Node<T>
where
  T: ShaderScalarOrVec,
  T::Item: ShaderIntType,
{
  type Output = Self;

  fn shr(self, rhs: Node<T::Shape<u32>>) -> Self::Output {
    OperatorNode::Binary {
      left: self.handle(),
      right: rhs.handle(),
      operator: BinaryOperator::ShiftRight,
    }
    .insert_api()
  }
}

impl<T> BitAnd for Node<T>
where
  T: ShaderScalarOrVec,
  T::Item: ShaderAndOrScalarType,
{
  type Output = Self;

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
  T: ShaderScalarOrVec,
  T::Item: ShaderAndOrScalarType,
{
  type Output = Self;

  fn bitor(self, rhs: Self) -> Self::Output {
    OperatorNode::Binary {
      left: self.handle(),
      right: rhs.handle(),
      operator: BinaryOperator::BitOr,
    }
    .insert_api()
  }
}

// note, we not impl the Not trait, because we have not impl for Node<bool>
impl<T> Node<T>
where
  T: ShaderScalarOrVec,
  T::Item: ShaderIntType,
{
  pub fn bitwise_not(self) -> Self {
    OperatorNode::Unary {
      one: self.handle(),
      operator: UnaryOperator::BitwiseNot,
    }
    .insert_api()
  }
}

impl<T> BitXor for Node<T>
where
  T: ShaderScalarOrVec,
  T::Item: ShaderIntType,
{
  type Output = Self;

  fn bitxor(self, rhs: Self) -> Self::Output {
    OperatorNode::Binary {
      left: self.handle(),
      right: rhs.handle(),
      operator: BinaryOperator::BitXor,
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

impl<T> Neg for Node<T>
where
  T: ShaderScalarOrVec,
  T::Item: ShaderSignedType,
{
  type Output = Self;
  fn neg(self) -> Self::Output {
    OperatorNode::Unary {
      one: self.handle(),
      operator: UnaryOperator::Neg,
    }
    .insert_api()
  }
}

impl<T> Node<T>
where
  T: ShaderScalarOrVec,
  T::Shape<bool>: PrimitiveShaderNodeType,
{
  pub fn equals(&self, other: impl Into<Self>) -> Node<T::Shape<bool>> {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.into().handle(),
      operator: BinaryOperator::Eq,
    }
    .insert_api()
  }

  pub fn not_equals(&self, other: impl Into<Self>) -> Node<T::Shape<bool>> {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.into().handle(),
      operator: BinaryOperator::NotEq,
    }
    .insert_api()
  }
}

impl<T> Node<T>
where
  T: ShaderScalarOrVec,
  T::Item: ShaderNumericScalarType,
  T::Shape<bool>: ShaderNodeType,
{
  pub fn less_than(&self, other: impl Into<Self>) -> Node<T::Shape<bool>> {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.into().handle(),
      operator: BinaryOperator::LessThan,
    }
    .insert_api()
  }
  pub fn less_equal_than(&self, other: impl Into<Self>) -> Node<T::Shape<bool>> {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.into().handle(),
      operator: BinaryOperator::LessEqualThan,
    }
    .insert_api()
  }
  pub fn greater_than(&self, other: impl Into<Self>) -> Node<T::Shape<bool>> {
    OperatorNode::Binary {
      left: self.handle(),
      right: other.into().handle(),
      operator: BinaryOperator::GreaterThan,
    }
    .insert_api()
  }
  pub fn greater_equal_than(&self, other: impl Into<Self>) -> Node<T::Shape<bool>> {
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

impl<T: ShaderVec<Item = bool>> Node<T> {
  /// component-wise logical negation
  #[must_use]
  pub fn not(&self) -> Self {
    OperatorNode::Unary {
      operator: UnaryOperator::LogicalNot,
      one: self.handle(),
    }
    .insert_api()
  }
}
