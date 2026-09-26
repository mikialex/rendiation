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

/// The operands of the component-wise binary operators(`+ - / %`), which are the same type
/// operands, or a numeric vector and a scalar of its component type in either order, where the
/// scalar applies to every component.
///
/// Unlike `*`, naga IR requires the same operand types for these operators, so the scalar operand
/// of the mixed form is splat to the vector before the operation.
///
/// see <https://www.w3.org/TR/WGSL/#arithmetic-expr>
pub trait ShaderComponentWiseOperands<Rhs>: ShaderNodeType {
  type Output: ShaderNodeType;
  fn unify(lhs: Node<Self>, rhs: Node<Rhs>) -> (Node<Self::Output>, Node<Self::Output>);
}

/// The valid operands of the `+` and `-` operators in WGSL: the numeric scalars and vectors (see
/// [ShaderComponentWiseOperands]), and the same type float matrices.
#[diagnostic::on_unimplemented(
  message = "`{Self}` and `{Rhs}` can not be added or subtracted in shader",
  note = "the operands must be the same type numeric scalar, vector or float matrix, or a numeric vector and a scalar of its component type"
)]
pub trait ShaderAddSub<Rhs>: ShaderComponentWiseOperands<Rhs> {}

/// The valid operands of the `/` and `%` operators in WGSL: the numeric scalars and vectors (see
/// [ShaderComponentWiseOperands]).
#[diagnostic::on_unimplemented(
  message = "`{Self}` can not be divided by `{Rhs}` in shader",
  note = "the operands must be the same type numeric scalar or vector, or a numeric vector and a scalar of its component type"
)]
pub trait ShaderDivRem<Rhs>: ShaderComponentWiseOperands<Rhs> {}

/// All the valid multiplications in WGSL: the component-wise multiplication, the scalar and vector
/// (or matrix) mixed multiplication, and the linear algebra matrix multiplication.
///
/// The math library's operator implementations are not reused, because they contain operations
/// that are invalid in WGSL, for example the homogeneous `Mat4 * Vec3`.
#[diagnostic::on_unimplemented(
  message = "`{Self}` can not be multiplied by `{Rhs}` in shader",
  note = "see the WGSL arithmetic expression overloads, the matrix element type must be f32 and the matrix vector dimensions must match"
)]
pub trait ShaderMul<Rhs>: ShaderNodeType {
  type Output: ShaderNodeType;
}

macro_rules! impl_shader_mul {
  ($lhs: ty, $rhs: ty, $output: ty) => {
    impl ShaderMul<$rhs> for $lhs {
      type Output = $output;
    }
  };
}

macro_rules! impl_same_type_operands {
  ($($ty: ty),+) => {
    $(
      impl ShaderComponentWiseOperands<$ty> for $ty {
        type Output = $ty;
        fn unify(lhs: Node<Self>, rhs: Node<Self>) -> (Node<Self>, Node<Self>) {
          (lhs, rhs)
        }
      }
      impl ShaderAddSub<$ty> for $ty {}
    )+
  };
}

macro_rules! impl_numeric_arithmetic {
  ($($ty: ty),+) => {
    $(
      impl_same_type_operands!($ty);
      impl ShaderDivRem<$ty> for $ty {}
      impl_shader_mul!($ty, $ty, $ty);
    )+
  };
}

macro_rules! impl_vector_scalar_arithmetic {
  ($scalar: ty, $($vec: ty),+) => {
    $(
      impl ShaderComponentWiseOperands<$scalar> for $vec {
        type Output = $vec;
        fn unify(lhs: Node<Self>, rhs: Node<$scalar>) -> (Node<$vec>, Node<$vec>) {
          (lhs, rhs.splat())
        }
      }
      impl ShaderComponentWiseOperands<$vec> for $scalar {
        type Output = $vec;
        fn unify(lhs: Node<Self>, rhs: Node<$vec>) -> (Node<$vec>, Node<$vec>) {
          (lhs.splat(), rhs)
        }
      }
      impl ShaderAddSub<$scalar> for $vec {}
      impl ShaderAddSub<$vec> for $scalar {}
      impl ShaderDivRem<$scalar> for $vec {}
      impl ShaderDivRem<$vec> for $scalar {}
      impl_shader_mul!($vec, $scalar, $vec);
      impl_shader_mul!($scalar, $vec, $vec);
    )+
  };
}

macro_rules! impl_scalar_vector_arithmetic {
  ($scalar: ty) => {
    impl_numeric_arithmetic!($scalar, Vec2<$scalar>, Vec3<$scalar>, Vec4<$scalar>);
    impl_vector_scalar_arithmetic!($scalar, Vec2<$scalar>, Vec3<$scalar>, Vec4<$scalar>);
  };
}

impl_scalar_vector_arithmetic!(f32);
impl_scalar_vector_arithmetic!(u32);
impl_scalar_vector_arithmetic!(i32);

macro_rules! impl_matrix_scalar_arithmetic {
  ($($mat: ty),+) => {
    $(
      impl_same_type_operands!($mat);
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

fn component_wise_binary<T, R>(
  lhs: Node<T>,
  rhs: Node<R>,
  operator: BinaryOperator,
) -> Node<T::Output>
where
  T: ShaderComponentWiseOperands<R>,
{
  let (left, right) = T::unify(lhs, rhs);
  OperatorNode::Binary {
    left: left.handle(),
    right: right.handle(),
    operator,
  }
  .insert_api()
}

impl<T, R> Add<Node<R>> for Node<T>
where
  T: ShaderAddSub<R>,
{
  type Output = Node<T::Output>;

  fn add(self, other: Node<R>) -> Self::Output {
    component_wise_binary(self, other, BinaryOperator::Add)
  }
}

impl<T, R> Sub<Node<R>> for Node<T>
where
  T: ShaderAddSub<R>,
{
  type Output = Node<T::Output>;

  fn sub(self, other: Node<R>) -> Self::Output {
    component_wise_binary(self, other, BinaryOperator::Sub)
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

impl<T, R> Div<Node<R>> for Node<T>
where
  T: ShaderDivRem<R>,
{
  type Output = Node<T::Output>;

  fn div(self, other: Node<R>) -> Self::Output {
    component_wise_binary(self, other, BinaryOperator::Div)
  }
}

impl<T, R> Rem<Node<R>> for Node<T>
where
  T: ShaderDivRem<R>,
{
  type Output = Node<T::Output>;

  fn rem(self, rhs: Node<R>) -> Self::Output {
    component_wise_binary(self, rhs, BinaryOperator::Rem)
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

impl<T, R> AddAssign<Node<R>> for Node<T>
where
  Self: Add<Node<R>, Output = Self>,
{
  fn add_assign(&mut self, rhs: Node<R>) {
    *self = *self + rhs;
  }
}

impl<T, R> SubAssign<Node<R>> for Node<T>
where
  Self: Sub<Node<R>, Output = Self>,
{
  fn sub_assign(&mut self, rhs: Node<R>) {
    *self = *self - rhs;
  }
}

impl<T, R> MulAssign<Node<R>> for Node<T>
where
  Self: Mul<Node<R>, Output = Self>,
{
  fn mul_assign(&mut self, rhs: Node<R>) {
    *self = *self * rhs;
  }
}

impl<T, R> DivAssign<Node<R>> for Node<T>
where
  Self: Div<Node<R>, Output = Self>,
{
  fn div_assign(&mut self, rhs: Node<R>) {
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
