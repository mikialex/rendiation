use rendiation_algebra::SquareMatrix;

use crate::*;

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
