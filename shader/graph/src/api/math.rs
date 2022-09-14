use rendiation_algebra::SquareMatrix;

use crate::*;

impl<T> Node<T>
where
  T: InnerProductSpace<f32> + PrimitiveShaderGraphNodeType,
{
  pub fn normalize(self) -> Self {
    ShaderGraphNodeExpr::Normalize(self.handle()).insert_graph()
  }
}

impl<T> Node<T>
where
  T: SquareMatrix<f32> + PrimitiveShaderGraphNodeType,
{
  pub fn inverse(self) -> Self {
    ShaderGraphNodeExpr::MatInverse(self.handle()).insert_graph()
  }
  pub fn transpose(self) -> Self {
    ShaderGraphNodeExpr::MatTranspose(self.handle()).insert_graph()
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
