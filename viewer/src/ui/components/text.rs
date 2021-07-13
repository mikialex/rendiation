use crate::{
  ui::{Component, Value},
  Presentable, PresentationBuilder, Primitive, Quad, UpdateCtx,
};

pub struct Text<T> {
  content: Value<String, T>,
}

impl<T> Into<Value<String, T>> for &str {
  fn into(self) -> Value<String, T> {
    Value::Static(self.to_owned())
  }
}

impl<T> Text<T> {
  pub fn new(content: impl Into<Value<String, T>>) -> Self {
    Self {
      content: content.into(),
    }
  }
}

impl<T> Component<T> for Text<T> {
  fn update(&mut self, model: &T, ctx: &mut UpdateCtx) {
    self.content.update(model);
  }
}

impl<T> Presentable for Text<T> {
  fn render(&self, builder: &mut PresentationBuilder) {
    builder.present.primitives.push(Primitive::Quad(Quad {
      x: 0.,
      y: 0.,
      width: 10.,
      height: 10.,
    }));
  }
}
