use rendiation_algebra::Vec4;

use crate::*;

pub struct Text<T> {
  content: Value<String, T>,
  position_computed: UIPosition,
  size_computed: LayoutSize,
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
      position_computed: Default::default(),
      size_computed: Default::default(),
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
      x: self.position_computed.x,
      y: self.position_computed.y,
      color: Vec4::new(0., 0., 0., 1.),
      font_size: 30.,
    }));
  }
}

impl<T> LayoutAble for Text<T> {
  fn layout(&mut self, constraint: LayoutConstraint) -> LayoutSize {
    self.size_computed = constraint.clamp(LayoutSize {
      width: (self.content.get().len() * 20) as f32,
      height: 30.,
    });
    self.size_computed
  }

  fn set_position(&mut self, position: UIPosition) {
    self.position_computed = position;
  }
}
