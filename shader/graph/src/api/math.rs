use rendiation_algebra::SquareMatrix;

use crate::*;

impl<T> Node<T>
where
  T: SquareMatrix<f32> + PrimitiveShaderGraphNodeType,
{
  pub fn inverse(self) -> Self {
    ShaderGraphNodeExpr::MatInverse(self.handle()).insert_graph()
  }
}
