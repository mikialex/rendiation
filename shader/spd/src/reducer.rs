use crate::*;

pub trait QuadReducer<T>: Copy + Clone + 'static {
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
