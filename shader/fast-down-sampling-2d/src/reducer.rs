use crate::*;

/// Reduce a 2x2 quad into one value.
///
/// The input is in row major order of the quad: (0, 0), (1, 0), (0, 1), (1, 1), where the first
/// component is x. All the quad reductions in this crate follow this order, so a reducer that is
/// not symmetric (for example a weighted filter) can rely on it. When used to reduce the multi
/// sample depth, the input is in sample index order.
pub trait QuadReducer<T>: std::any::Any {
  fn reduce(&self, v: [Node<T>; 4]) -> Node<T>;
}

#[derive(Clone, Copy)]
pub struct MaxReducer;
impl<T> QuadReducer<T> for MaxReducer
where
  T: ShaderScalarOrVec,
  T::Item: ShaderNumericScalarType,
{
  fn reduce(&self, v: [Node<T>; 4]) -> Node<T> {
    let [v1, v2, v3, v4] = v;
    v1.max(v2).max(v3).max(v4)
  }
}

#[derive(Clone, Copy)]
pub struct MinReducer;
impl<T> QuadReducer<T> for MinReducer
where
  T: ShaderScalarOrVec,
  T::Item: ShaderNumericScalarType,
{
  fn reduce(&self, v: [Node<T>; 4]) -> Node<T> {
    let [v1, v2, v3, v4] = v;
    v1.min(v2).min(v3).min(v4)
  }
}

#[derive(Clone, Copy)]
pub struct MipMapReducer;
impl QuadReducer<Vec4<f32>> for MipMapReducer {
  fn reduce(&self, v: [Node<Vec4<f32>>; 4]) -> Node<Vec4<f32>> {
    (v[0] + v[1] + v[2] + v[3]) * val(0.25)
  }
}
