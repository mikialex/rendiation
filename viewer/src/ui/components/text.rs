use rendiation_algebra::Vec4;

use crate::{
  ui::{Component, Value},
  Presentable, PresentationBuilder, Primitive, TextInfo, UpdateCtx,
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
    builder.present.primitives.push(Primitive::Text(TextInfo {
      content: "test".to_owned(),
      max_width: Some(100.),
      x: 100.,
      y: 100.,
      color: Vec4::new(0., 0., 0., 1.),
      font_size: 30.,
    }));
  }
}
