use crate::*;

/// The optional value in shader, we do not have sum type(enum) in shader, so the payload always
/// exists, but it is only meaningful when `is_some` is true.
#[derive(Clone, Copy)]
pub struct ShaderOption<T> {
  pub is_some: Node<bool>,
  pub payload: T,
}

impl<T> ShaderOption<T> {
  pub fn new(is_some: Node<bool>, payload: T) -> Self {
    Self { is_some, payload }
  }

  pub fn map<U>(self, f: impl FnOnce(T) -> U) -> ShaderOption<U> {
    ShaderOption::new(self.is_some, f(self.payload))
  }
}

impl<T: ShaderSizedValueNodeType> ShaderOption<Node<T>> {
  /// the payload if it is some, otherwise the default value
  pub fn unwrap_or(self, default: impl Into<Node<T>>) -> Node<T> {
    let default = default.into();
    self.is_some.select_branched(|| self.payload, || default)
  }
}
