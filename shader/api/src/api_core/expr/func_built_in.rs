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
impl Node<f32> {
  pub fn is_nan(&self) -> Node<bool> {
    make_builtin_call(ShaderBuiltInFunction::IsNan, [self.handle()])
  }
}

impl<T> Node<T>
where
  T: ShaderVec + PrimitiveShaderNodeType,
  T::Item: ShaderFloatType,
{
  pub fn normalize(self) -> Self {
    make_builtin_call(ShaderBuiltInFunction::Normalize, [self.handle()])
  }

  pub fn dot(self, other: impl Into<Self>) -> Node<T::Item> {
    make_builtin_call(
      ShaderBuiltInFunction::Dot,
      [self.handle(), other.into().handle()],
    )
  }

  /// return `incident_direction - 2 * dot(self, incident_direction) * self`.
  pub fn reflect(self, incident_direction: impl Into<Self>) -> Self {
    make_builtin_call(
      ShaderBuiltInFunction::Reflect,
      [incident_direction.into().handle(), self.handle()],
    )
  }

  /// For the incident vector (incident_direction) and surface normal (self), and the ratio of indices
  /// of refraction (ior), let `k = 1.0 - ior * ior * (1.0 - dot(self, incident_direction) * dot(self, incident_direction))`.
  /// If `k < 0.0`, returns the refraction vector 0.0, otherwise return the refraction
  /// vector `ior * incident_direction - (ior * dot(self, incident_direction) + sqrt(k)) * self`. The incident_direction
  /// and the normal (self) should be normalized for desired results according to Snell’s Law;
  /// otherwise, the results may not conform to expected physical behavior.
  pub fn refract(self, incident_direction: impl Into<Self>, ior: impl Into<Node<T::Item>>) -> Self {
    make_builtin_call(
      ShaderBuiltInFunction::Refract,
      [
        incident_direction.into().handle(),
        self.handle(),
        ior.into().handle(),
      ],
    )
  }
}

impl<T: ShaderFloatType> Node<Vec3<T>> {
  pub fn cross(self, other: impl Into<Self>) -> Node<Vec3<T>> {
    make_builtin_call(
      ShaderBuiltInFunction::Cross,
      [self.handle(), other.into().handle()],
    )
  }
}

impl<T> Node<T>
where
  T: ShaderScalarOrVec,
  T::Item: ShaderFloatType,
{
  /// Evaluates to the absolute value of self if T is scalar.
  pub fn length(self) -> Node<f32> {
    make_builtin_call(ShaderBuiltInFunction::Length, [self.handle()])
  }

  /// `(self - other).length`
  pub fn distance(self, other: impl Into<Self>) -> Node<T::Item> {
    make_builtin_call(
      ShaderBuiltInFunction::Distance,
      [self.handle(), other.into().handle()],
    )
  }
}

impl<T> Node<T>
where
  T: ShaderScalarOrVec,
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
  /// `self.clamp(0.0, 1.0)`
  pub fn saturate(self) -> Self {
    make_builtin_call(ShaderBuiltInFunction::Saturate, [self.handle()])
  }
}

impl<T> Node<T>
where
  T: ShaderScalarOrVec,
  T::Item: ShaderFloatType,
{
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

  pub fn smoothstep_per_channel(self, low: impl Into<Self>, high: impl Into<Self>) -> Self {
    make_builtin_call(
      ShaderBuiltInFunction::SmoothStep,
      [low.into().handle(), high.into().handle(), self.handle()],
    )
  }

  pub fn mix_per_channel(self, low: impl Into<Self>, high: impl Into<Self>) -> Self {
    make_builtin_call(
      ShaderBuiltInFunction::Mix,
      [low.into().handle(), high.into().handle(), self.handle()],
    )
  }

  /// e^self
  pub fn exp(self) -> Self {
    make_builtin_call(ShaderBuiltInFunction::Exp, [self.handle()])
  }
  /// 2^self
  pub fn exp2(self) -> Self {
    make_builtin_call(ShaderBuiltInFunction::Exp2, [self.handle()])
  }
  /// e based
  pub fn ln(self) -> Self {
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
}

impl<T> Node<T>
where
  T: ShaderFloatType,
{
  pub fn smoothstep<V>(self, low: impl Into<Node<V>>, high: impl Into<Node<V>>) -> Node<V>
  where
    V: ShaderScalarOrVec,
    V::Item: ShaderFloatType,
  {
    make_builtin_call(
      ShaderBuiltInFunction::SmoothStep,
      [low.into().handle(), high.into().handle(), self.handle()],
    )
  }

  pub fn mix<V>(self, low: impl Into<Node<V>>, high: impl Into<Node<V>>) -> Node<V>
  where
    V: ShaderScalarOrVec,
    V::Item: ShaderFloatType,
  {
    make_builtin_call(
      ShaderBuiltInFunction::Mix,
      [low.into().handle(), high.into().handle(), self.handle()],
    )
  }
}

impl<T: ShaderScalarOrVec> Node<T> {
  pub fn abs(self) -> Self {
    make_builtin_call(ShaderBuiltInFunction::Abs, [self.handle()])
  }
}

impl<T> Node<T>
where
  T: ShaderScalarOrVec,
  T::Item: ShaderSignedType,
{
  /// return per component 1 when self>0, 0 when self==0, -1 when self<0
  pub fn sign(self) -> Self {
    make_builtin_call(ShaderBuiltInFunction::Sign, [self.handle()])
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
  pub fn forward(self) -> Node<Vec3<f32>> {
    self.nth_colum(2).xyz()
  }
  pub fn scale(self) -> Node<Vec3<f32>> {
    let x = self.nth_colum(0).length();
    let y = self.nth_colum(1).length();
    let z = self.nth_colum(2).length();
    (x, y, z).into()
  }
  pub fn nth_colum(self, n: u32) -> Node<Vec4<f32>> {
    unsafe { index_access_field(self.handle(), n as usize).into_node() }
  }
}

impl<Bools: ShaderScalarOrVec<Item = bool>> Node<Bools> {
  pub fn select_per_channel<T: ShaderScalarType>(
    &self,
    true_case: impl Into<Node<Bools::Container<T>>>,
    false_case: impl Into<Node<Bools::Container<T>>>,
  ) -> Node<Bools::Container<T>>
  where
    T: ShaderScalarType,
    Bools::Container<T>: ShaderNodeType,
  {
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

impl Node<bool> {
  pub fn select<T: ShaderAnyScalarOrVec>(
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

impl<T: ShaderScalarOrVec<Item = bool>> Node<T> {
  pub fn all(self) -> Node<bool> {
    make_builtin_call(ShaderBuiltInFunction::All, [self.handle()])
  }
  pub fn any(self) -> Node<bool> {
    make_builtin_call(ShaderBuiltInFunction::Any, [self.handle()])
  }
}

/// in wgsl spec, the item must be f32, not [ShaderFloatType]
impl<T: ShaderScalarOrVec<Item = f32> + ShaderNodeType> Node<T> {
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

impl<T> Node<T>
where
  T: ShaderScalarOrVec,
  T::Item: ShaderFloatType,
{
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

  pub fn sinh(self) -> Node<T> {
    make_builtin_call(ShaderBuiltInFunction::Sinh, [self.handle()])
  }
  pub fn cosh(self) -> Node<T> {
    make_builtin_call(ShaderBuiltInFunction::Cosh, [self.handle()])
  }
  pub fn tanh(self) -> Node<T> {
    make_builtin_call(ShaderBuiltInFunction::Tanh, [self.handle()])
  }

  pub fn asinh(self) -> Node<T> {
    make_builtin_call(ShaderBuiltInFunction::Asinh, [self.handle()])
  }
  pub fn acosh(self) -> Node<T> {
    make_builtin_call(ShaderBuiltInFunction::Acosh, [self.handle()])
  }
  pub fn atanh(self) -> Node<T> {
    make_builtin_call(ShaderBuiltInFunction::Atanh, [self.handle()])
  }

  /// Returns 1.0 if edge ≤ x, and 0.0 otherwise
  pub fn step(self, edge: Node<T>) -> Node<T> {
    make_builtin_call(ShaderBuiltInFunction::Step, [self.handle(), edge.handle()])
  }
}

impl<T> Node<T>
where
  T: ShaderScalarOrVec,
  T::Item: ShaderIntType,
{
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

impl<T> Node<T>
where
  T: ShaderScalarOrVec,
  T::Item: ShaderFloatType,
{
  pub fn frexp(self) -> (Node<T::Container<T::Item>>, Node<i32>) {
    let raw = make_builtin_call_with_ty_helper::<AnyType>(
      ShaderBuiltInFunction::Frexp,
      T::primitive_ty(),
      vec![self.handle()],
    )
    .handle();

    unsafe {
      let fr = index_access_field(raw, 0).into_node();
      let exp = index_access_field(raw, 1).into_node();
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

impl Node<Vec4<f32>> {
  pub fn pack4x8snorm(self) -> Node<u32> {
    make_builtin_call(ShaderBuiltInFunction::Pack4x8snorm, [self.handle()])
  }
  pub fn pack4x8unorm(self) -> Node<u32> {
    make_builtin_call(ShaderBuiltInFunction::Pack4x8unorm, [self.handle()])
  }
}

impl Node<Vec2<f32>> {
  pub fn pack2x16snorm(self) -> Node<u32> {
    make_builtin_call(ShaderBuiltInFunction::Pack2x16snorm, [self.handle()])
  }
  pub fn pack2x16unorm(self) -> Node<u32> {
    make_builtin_call(ShaderBuiltInFunction::Pack2x16unorm, [self.handle()])
  }
  pub fn pack2x16float(self) -> Node<u32> {
    make_builtin_call(ShaderBuiltInFunction::Pack2x16float, [self.handle()])
  }
}

impl Node<u32> {
  pub fn unpack4x8snorm(self) -> Node<Vec4<f32>> {
    make_builtin_call(ShaderBuiltInFunction::Unpack4x8snorm, [self.handle()])
  }
  pub fn unpack4x8unorm(self) -> Node<Vec4<f32>> {
    make_builtin_call(ShaderBuiltInFunction::Unpack4x8unorm, [self.handle()])
  }

  pub fn unpack2x16snorm(self) -> Node<Vec2<f32>> {
    make_builtin_call(ShaderBuiltInFunction::Unpack2x16snorm, [self.handle()])
  }
  pub fn unpack2x16unorm(self) -> Node<Vec2<f32>> {
    make_builtin_call(ShaderBuiltInFunction::Unpack2x16unorm, [self.handle()])
  }
  pub fn unpack2x16float(self) -> Node<Vec2<f32>> {
    make_builtin_call(ShaderBuiltInFunction::Unpack2x16float, [self.handle()])
  }
}
