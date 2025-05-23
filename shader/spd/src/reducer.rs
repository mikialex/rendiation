use crate::*;

pub trait QuadReducer<T> {
  fn reduce(&self, v: [Node<T>; 4]) -> Node<T>;
}

#[derive(Clone, Copy)]
pub struct MaxReducer;
impl<T: PrimitiveShaderNodeType> QuadReducer<T> for MaxReducer {
  fn reduce(&self, v: [Node<T>; 4]) -> Node<T> {
    let [v1, v2, v3, v4] = v;
    v1.max(v2).max(v3).max(v4)
  }
}

#[derive(Clone, Copy)]
pub struct MipMapReducer;
impl QuadReducer<Vec4<f32>> for MipMapReducer {
  fn reduce(&self, v: [Node<Vec4<f32>>; 4]) -> Node<Vec4<f32>> {
    (v[0] + v[1] + v[2] + v[3]) * val(0.25)
  }
}
