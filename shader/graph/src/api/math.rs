use rendiation_algebra::SquareMatrix;

use crate::*;

pub fn make_builtin_call<T: ShaderGraphNodeType>(
  ty: ShaderBuiltInFunction,
  params: impl IntoIterator<Item = ShaderGraphNodeRawHandle>,
) -> Node<T> {
  ShaderGraphNodeExpr::FunctionCall {
    meta: ShaderFunctionType::BuiltIn(ty),
    parameters: params.into_iter().collect(),
  }
  .insert_graph()
}

impl<T> Node<T>
where
  T: InnerProductSpace<f32> + PrimitiveShaderGraphNodeType,
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
  T: PrimitiveShaderGraphNodeType, /* where
                                    * T: RealVector<f32> + PrimitiveShaderGraphNodeType, */
{
  pub fn min(self, other: Self) -> Self {
    make_builtin_call(ShaderBuiltInFunction::Min, [self.handle(), other.handle()])
  }
  pub fn max(self, other: Self) -> Self {
    make_builtin_call(ShaderBuiltInFunction::Max, [self.handle(), other.handle()])
  }
  pub fn clamp(self, min: Self, max: Self) -> Self {
    make_builtin_call(
      ShaderBuiltInFunction::Clamp,
      [self.handle(), min.handle(), max.handle()],
    )
  }
}

// adhoc component-wise compute
impl<T> Node<T>
where
  T: PrimitiveShaderGraphNodeType,
{
  pub fn abs(self) -> Self {
    make_builtin_call(ShaderBuiltInFunction::Abs, [self.handle()])
  }
  pub fn pow(self, e: Node<f32>) -> Self {
    make_builtin_call(ShaderBuiltInFunction::Pow, [self.handle(), e.handle()])
  }
  pub fn saturate(self) -> Self {
    make_builtin_call(ShaderBuiltInFunction::Saturate, [self.handle()])
  }
}

impl<T> Node<T>
where
  T: Lerp<T> + PrimitiveShaderGraphNodeType,
{
  pub fn smoothstep(self, low: Self, high: Self) -> Self {
    make_builtin_call(
      ShaderBuiltInFunction::SmoothStep,
      [low.handle(), high.handle(), self.handle()],
    )
  }
}

impl<T> Node<T>
where
  T: SquareMatrix<f32> + PrimitiveShaderGraphNodeType,
{
  pub fn transpose(self) -> Self {
    make_builtin_call(ShaderBuiltInFunction::MatTranspose, [self.handle()])
  }
}

impl Node<Mat4<f32>> {
  pub fn position(self) -> Node<Vec3<f32>> {
    self.nth_colum(3).xyz()
  }
  pub fn nth_colum(self, n: impl Into<Node<i32>>) -> Node<Vec4<f32>> {
    ShaderGraphNodeExpr::Operator(OperatorNode::Index {
      array: self.handle(),
      entry: n.into().handle(),
    })
    .insert_graph()
  }
}

impl Node<bool> {
  pub fn select<T: ShaderGraphNodeType>(&self, true_case: Node<T>, false_case: Node<T>) -> Node<T> {
    make_builtin_call(
      ShaderBuiltInFunction::Select,
      [false_case.handle(), true_case.handle(), self.handle()],
    )
  }
}

// todo restrict
impl<T> Node<T> {
  pub fn all(self) -> Node<bool> {
    todo!()
  }
  pub fn any(self) -> Node<bool> {
    todo!()
  }
}

// todo restrict
impl<T> Node<T> {
  pub fn dpdx(self) -> Node<T> {
    todo!()
  }
  pub fn dpdy(self) -> Node<T> {
    todo!()
  }
  pub fn dpdx_fine(self) -> Node<T> {
    todo!()
  }
  pub fn dpdy_fine(self) -> Node<T> {
    todo!()
  }
  pub fn dpdx_coarse(self) -> Node<T> {
    todo!()
  }
  pub fn dpdy_coarse(self) -> Node<T> {
    todo!()
  }
  pub fn fwidth(self) -> Node<T> {
    todo!()
  }
  pub fn fwidth_fine(self) -> Node<T> {
    todo!()
  }
  pub fn fwidth_coarse(self) -> Node<T> {
    todo!()
  }

  pub fn sqrt(self) -> Node<T> {
    todo!()
  }
  pub fn inverse_sqrt(self) -> Node<T> {
    todo!()
  }

  pub fn sin(self) -> Node<T> {
    todo!()
  }
  pub fn cos(self) -> Node<T> {
    todo!()
  }
  pub fn tan(self) -> Node<T> {
    todo!()
  }

  pub fn fract(self) -> Node<T> {
    todo!()
  }
}
