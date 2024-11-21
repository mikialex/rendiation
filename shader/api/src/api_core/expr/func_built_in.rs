use rendiation_algebra::SquareMatrix;

use crate::*;

#[derive(Clone, Copy, Debug, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub enum ShaderBuiltInFunction {
  Select,
  // relational
  All,
  Any,
  IsNan,
  IsInf,
  // comparison
  Abs,
  Min,
  Max,
  Clamp,
  Saturate,
  // trigonometry
  Cos,
  Cosh,
  Sin,
  Sinh,
  Tan,
  Tanh,
  Acos,
  Asin,
  Atan,
  Atan2,
  Asinh,
  Acosh,
  Atanh,
  Radians,
  Degrees,
  // decomposition
  Ceil,
  Floor,
  Round,
  Fract,
  Trunc,
  Modf,
  Frexp,
  Ldexp,
  // exponent
  Exp,
  Exp2,
  Log,
  Log2,
  Pow,
  // geometry
  Dot,
  Outer,
  Cross,
  Distance,
  Length,
  Normalize,
  FaceForward,
  Reflect,
  Refract,
  // computational
  Sign,
  Fma,
  Mix,
  Step,
  SmoothStep,
  Sqrt,
  InverseSqrt,
  Inverse,
  Transpose,
  Determinant,
  // bits
  CountTrailingZeros,
  CountLeadingZeros,
  CountOneBits,
  ReverseBits,
  ExtractBits,
  InsertBits,
  FindLsb,
  FindMsb,
  // data packing
  Pack4x8snorm,
  Pack4x8unorm,
  Pack2x16snorm,
  Pack2x16unorm,
  Pack2x16float,
  // data unpacking
  Unpack4x8snorm,
  Unpack4x8unorm,
  Unpack2x16snorm,
  Unpack2x16unorm,
  Unpack2x16float,
  // array extra
  ArrayLength,
}

pub fn make_builtin_call<T: ShaderNodeType>(
  ty: ShaderBuiltInFunction,
  params: impl IntoIterator<Item = ShaderNodeRawHandle>,
) -> Node<T> {
  ShaderNodeExpr::FunctionCall {
    meta: ShaderFunctionType::BuiltIn {
      ty,
      ty_help_info: None,
    },
    parameters: params.into_iter().collect(),
  }
  .insert_api()
}

pub fn make_builtin_call_with_ty_helper<T: ShaderNodeType>(
  ty: ShaderBuiltInFunction,
  ty_help_info: PrimitiveShaderValueType,
  params: impl IntoIterator<Item = ShaderNodeRawHandle>,
) -> Node<T> {
  ShaderNodeExpr::FunctionCall {
    meta: ShaderFunctionType::BuiltIn {
      ty,
      ty_help_info: Some(ty_help_info),
    },
    parameters: params.into_iter().collect(),
  }
  .insert_api()
}

impl<T> Node<T>
where
  T: InnerProductSpace<f32> + PrimitiveShaderNodeType,
{
  pub fn normalize(self) -> Self {
    make_builtin_call(ShaderBuiltInFunction::Normalize, [self.handle()])
  }

  pub fn length(self) -> Node<f32> {
    make_builtin_call(ShaderBuiltInFunction::Length, [self.handle()])
  }

  pub fn dot(self, other: impl Into<Self>) -> Node<f32> {
    make_builtin_call(
      ShaderBuiltInFunction::Dot,
      [self.handle(), other.into().handle()],
    )
  }

  pub fn cross(self, other: impl Into<Self>) -> Node<Vec3<f32>> {
    make_builtin_call(
      ShaderBuiltInFunction::Cross,
      [self.handle(), other.into().handle()],
    )
  }
}

impl<T> Node<T>
where
  T: PrimitiveShaderNodeType, /* where
                               * T: RealVector<f32> + PrimitiveShaderNodeType, */
{
  pub fn min(self, other: impl Into<Self>) -> Self {
    make_builtin_call(
      ShaderBuiltInFunction::Min,
      [self.handle(), other.into().handle()],
    )
  }
  pub fn max(self, other: impl Into<Self>) -> Self {
    make_builtin_call(
      ShaderBuiltInFunction::Max,
      [self.handle(), other.into().handle()],
    )
  }
  pub fn clamp(self, min: impl Into<Self>, max: impl Into<Self>) -> Self {
    make_builtin_call(
      ShaderBuiltInFunction::Clamp,
      [self.handle(), min.into().handle(), max.into().handle()],
    )
  }
}

// adhoc component-wise compute
impl<T> Node<T>
where
  T: PrimitiveShaderNodeType,
{
  pub fn abs(self) -> Self {
    make_builtin_call(ShaderBuiltInFunction::Abs, [self.handle()])
  }

  pub fn sign(self) -> Self {
    make_builtin_call(ShaderBuiltInFunction::Sign, [self.handle()])
  }

  /// e^self
  pub fn exp(self) -> Self {
    make_builtin_call(ShaderBuiltInFunction::Exp, [self.handle()])
  }
  /// 2^self
  pub fn exp2(self) -> Self {
    make_builtin_call(ShaderBuiltInFunction::Exp2, [self.handle()])
  }
  /// e based, ln(self)
  pub fn log(self) -> Self {
    make_builtin_call(ShaderBuiltInFunction::Log, [self.handle()])
  }
  /// 2 based, log(2, self)
  pub fn log2(self) -> Self {
    make_builtin_call(ShaderBuiltInFunction::Log2, [self.handle()])
  }
  /// self^e
  pub fn pow(self, e: impl Into<Self>) -> Self {
    make_builtin_call(
      ShaderBuiltInFunction::Pow,
      [self.handle(), e.into().handle()],
    )
  }

  pub fn saturate(self) -> Self {
    make_builtin_call(ShaderBuiltInFunction::Saturate, [self.handle()])
  }
}

// todo fix bound
impl<T> Node<T>
where
  T: PrimitiveShaderNodeType,
{
  pub fn smoothstep(self, low: impl Into<Self>, high: impl Into<Self>) -> Self {
    make_builtin_call(
      ShaderBuiltInFunction::SmoothStep,
      [low.into().handle(), high.into().handle(), self.handle()],
    )
  }
}

// todo fix bound
impl<T: PrimitiveShaderNodeType> Node<T> {
  pub fn mix<V: PrimitiveShaderNodeType>(self, low: Node<V>, high: Node<V>) -> Node<V> {
    make_builtin_call(
      ShaderBuiltInFunction::Mix,
      [low.handle(), high.handle(), self.handle()],
    )
  }
}

impl<T> Node<T>
where
  T: SquareMatrix<f32> + PrimitiveShaderNodeType,
{
  pub fn transpose(self) -> Self {
    make_builtin_call(ShaderBuiltInFunction::Transpose, [self.handle()])
  }
}

impl Node<Mat4<f32>> {
  pub fn position(self) -> Node<Vec3<f32>> {
    self.nth_colum(3).xyz()
  }
  pub fn nth_colum(self, n: u32) -> Node<Vec4<f32>> {
    unsafe { index_access_field(self.handle(), n as usize) }
  }
}

impl Node<bool> {
  // todo, support component wise select
  pub fn select<T: ShaderNodeType>(
    &self,
    true_case: impl Into<Node<T>>,
    false_case: impl Into<Node<T>>,
  ) -> Node<T> {
    make_builtin_call(
      ShaderBuiltInFunction::Select,
      [
        false_case.into().handle(),
        true_case.into().handle(),
        self.handle(),
      ],
    )
  }
}

pub trait BoolLikeShaderNodeType {}
impl BoolLikeShaderNodeType for bool {}
impl BoolLikeShaderNodeType for Vec2<bool> {}
impl BoolLikeShaderNodeType for Vec3<bool> {}
impl BoolLikeShaderNodeType for Vec4<bool> {}
impl<T: BoolLikeShaderNodeType> Node<T> {
  pub fn all(self) -> Node<bool> {
    make_builtin_call(ShaderBuiltInFunction::All, [self.handle()])
  }
  pub fn any(self) -> Node<bool> {
    make_builtin_call(ShaderBuiltInFunction::Any, [self.handle()])
  }
}

// todo restrict
impl<T: ShaderNodeType> Node<T> {
  pub fn derivative(self, axis: DerivativeAxis, ctrl: DerivativeControl) -> Node<T> {
    ShaderNodeExpr::Derivative {
      axis,
      ctrl,
      source: self.handle(),
    }
    .insert_api()
  }

  pub fn dpdx(self) -> Node<T> {
    self.derivative(DerivativeAxis::X, DerivativeControl::None)
  }
  pub fn dpdy(self) -> Node<T> {
    self.derivative(DerivativeAxis::Y, DerivativeControl::None)
  }
  pub fn dpdx_fine(self) -> Node<T> {
    self.derivative(DerivativeAxis::X, DerivativeControl::Fine)
  }
  pub fn dpdy_fine(self) -> Node<T> {
    self.derivative(DerivativeAxis::Y, DerivativeControl::Fine)
  }
  pub fn dpdx_coarse(self) -> Node<T> {
    self.derivative(DerivativeAxis::X, DerivativeControl::Coarse)
  }
  pub fn dpdy_coarse(self) -> Node<T> {
    self.derivative(DerivativeAxis::Y, DerivativeControl::Coarse)
  }
  pub fn fwidth(self) -> Node<T> {
    self.derivative(DerivativeAxis::Width, DerivativeControl::None)
  }
  pub fn fwidth_fine(self) -> Node<T> {
    self.derivative(DerivativeAxis::Width, DerivativeControl::Fine)
  }
  pub fn fwidth_coarse(self) -> Node<T> {
    self.derivative(DerivativeAxis::Width, DerivativeControl::Coarse)
  }
}

// todo restrict
impl<T: ShaderNodeType> Node<T> {
  pub fn sqrt(self) -> Node<T> {
    make_builtin_call(ShaderBuiltInFunction::Sqrt, [self.handle()])
  }
  pub fn inverse_sqrt(self) -> Node<T> {
    make_builtin_call(ShaderBuiltInFunction::InverseSqrt, [self.handle()])
  }

  pub fn sin(self) -> Node<T> {
    make_builtin_call(ShaderBuiltInFunction::Sin, [self.handle()])
  }
  pub fn cos(self) -> Node<T> {
    make_builtin_call(ShaderBuiltInFunction::Cos, [self.handle()])
  }
  pub fn tan(self) -> Node<T> {
    make_builtin_call(ShaderBuiltInFunction::Tan, [self.handle()])
  }
  pub fn asin(self) -> Node<T> {
    make_builtin_call(ShaderBuiltInFunction::Asin, [self.handle()])
  }
  pub fn acos(self) -> Node<T> {
    make_builtin_call(ShaderBuiltInFunction::Acos, [self.handle()])
  }
  pub fn atan(self) -> Node<T> {
    make_builtin_call(ShaderBuiltInFunction::Atan, [self.handle()])
  }
  pub fn atan2(self, other: Node<T>) -> Node<T> {
    make_builtin_call(
      ShaderBuiltInFunction::Atan2,
      [self.handle(), other.handle()],
    )
  }

  pub fn ceil(self) -> Node<T> {
    make_builtin_call(ShaderBuiltInFunction::Ceil, [self.handle()])
  }
  pub fn floor(self) -> Node<T> {
    make_builtin_call(ShaderBuiltInFunction::Floor, [self.handle()])
  }
  pub fn round(self) -> Node<T> {
    make_builtin_call(ShaderBuiltInFunction::Round, [self.handle()])
  }
  pub fn fract(self) -> Node<T> {
    make_builtin_call(ShaderBuiltInFunction::Fract, [self.handle()])
  }
  pub fn trunc(self) -> Node<T> {
    make_builtin_call(ShaderBuiltInFunction::Trunc, [self.handle()])
  }

  pub fn extract_bits(self, offset: Node<u32>, count: Node<u32>) -> Node<T> {
    make_builtin_call(
      ShaderBuiltInFunction::ExtractBits,
      [self.handle(), offset.handle(), count.handle()],
    )
  }
  pub fn insert_bits(self, new_bits: Node<T>, offset: Node<u32>, count: Node<u32>) -> Node<T> {
    make_builtin_call(
      ShaderBuiltInFunction::InsertBits,
      [
        self.handle(),
        new_bits.handle(),
        offset.handle(),
        count.handle(),
      ],
    )
  }
}

pub trait ExpDecomposableShaderNodeType: PrimitiveShaderNodeType {
  type Fract: PrimitiveShaderNodeType;
  type Exp: PrimitiveShaderNodeType;
}
impl ExpDecomposableShaderNodeType for f32 {
  type Fract = f32;
  type Exp = i32;
}
macro_rules! impl_exp_decompose {
  ($t: tt) => {
    impl ExpDecomposableShaderNodeType for $t<f32> {
      type Fract = $t<f32>;
      type Exp = $t<i32>;
    }
  };
}
impl_exp_decompose!(Vec2);
impl_exp_decompose!(Vec3);
impl_exp_decompose!(Vec4);

impl<T: ExpDecomposableShaderNodeType> Node<T> {
  pub fn frexp(self) -> (Node<T::Fract>, Node<T::Exp>) {
    let raw = make_builtin_call_with_ty_helper::<AnyType>(
      ShaderBuiltInFunction::Frexp,
      T::PRIMITIVE_TYPE,
      vec![self.handle()],
    )
    .handle();

    unsafe {
      let fr = index_access_field(raw, 0);
      let exp = index_access_field(raw, 1);
      (fr, exp)
    }
  }
}

// todo expand to more type
impl Node<Vec3<f32>> {
  pub fn max_channel(self) -> Node<f32> {
    self.x().max(self.y()).max(self.z())
  }
}
impl Node<Vec3<f32>> {
  pub fn min_channel(self) -> Node<f32> {
    self.x().min(self.y()).min(self.z())
  }
}
