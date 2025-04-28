use crate::*;

pub trait QuadReducer<T>: Copy + Clone + 'static {
  fn reduce(&self, v: &ShaderPtrOf<[T; 4]>) -> Node<T>;
}

#[derive(Clone, Copy)]
pub struct MaxReducer;
impl<T: PrimitiveShaderNodeType> QuadReducer<T> for MaxReducer {
  fn reduce(&self, v: &ShaderPtrOf<[T; 4]>) -> Node<T> {
    let v1 = v.index(0).load();
    let v2 = v.index(1).load();
    let v3 = v.index(2).load();
    let v4 = v.index(3).load();
    v1.max(v2).max(v3).max(v4)
  }
}
