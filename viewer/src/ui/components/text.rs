use crate::ui::{Component, Value};

pub struct Text<T> {
  content: Value<String, T>,
}

impl<T> Into<Value<String, T>> for &str {
  fn into(self) -> Value<String, T> {
    todo!()
  }
}

impl<T> Text<T> {
  pub fn new(content: impl Into<Value<String, T>>) -> Self {
    todo!()
  }
}

impl<T> Component<T> for Text<T> {
  fn update(&mut self, model: &T) {
    self.content.update(model);
  }
}
