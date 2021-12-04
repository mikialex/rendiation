use crate::*;

mod cursor;
pub use cursor::*;

mod editable;
pub use editable::*;

pub struct Text {
  pub content: LayoutSource<String>,
  pub line_wrap: LineWrap,
  pub horizon_align: HorizontalAlignment,
  pub vertical_align: VerticalAlignment,
  pub text_layout: Option<TextLayoutRef>,
  pub layout: LayoutUnit,
}

impl Default for Text {
  fn default() -> Self {
    Self {
      content: LayoutSource::new("".into()),
      layout: Default::default(),
      horizon_align: Default::default(),
      vertical_align: Default::default(),
      line_wrap: Default::default(),
      text_layout: None,
    }
  }
}

impl Text {
  pub fn new(content: impl Into<String>) -> Self {
    Self {
      content: LayoutSource::new(content.into()),
      ..Default::default()
    }
  }

  // todo, put it in setters
  pub fn reset_text_layout(&mut self) {
    self.text_layout = None;
  }

  pub fn with_line_wrap(mut self, line_wrap: LineWrap) -> Self {
    self.line_wrap = line_wrap;
    self
  }

  pub fn with_horizon_align(mut self, horizon_align: HorizontalAlignment) -> Self {
    self.horizon_align = horizon_align;
    self
  }

  pub fn with_vertical_align(mut self, vertical_align: VerticalAlignment) -> Self {
    self.vertical_align = vertical_align;
    self
  }

  pub(super) fn get_text_layout(
    &mut self,
    fonts: &FontManager,
    text: &mut TextCache,
  ) -> &TextLayoutRef {
    self.text_layout.get_or_insert_with(|| {
      let text_info = TextInfo {
        content: self.content.get().clone(),
        bounds: self.layout.size,
        line_wrap: self.line_wrap,
        horizon_align: self.horizon_align,
        vertical_align: self.vertical_align,
        x: self.layout.absolute_position.x,
        y: self.layout.absolute_position.y,
        color: (0., 0., 0., 1.).into(),
        font_size: 30.,
      };

      text.cache_layout(&text_info, fonts)
    })
  }
}

impl<T> Component<T> for Text {
  fn update(&mut self, _: &T, ctx: &mut UpdateCtx) {
    self.layout.check_attach(ctx);
    if self.content.changed() {
      self.reset_text_layout();
    }
    self.content.refresh(&mut self.layout, ctx);
  }
}

impl Presentable for Text {
  fn render(&mut self, builder: &mut PresentationBuilder) {
    self.layout.update_world(builder.current_origin_offset);

    builder.present.primitives.push(Primitive::Text(
      self.get_text_layout(builder.fonts, builder.texts).clone(),
    ));

    builder.present.primitives.push(Primitive::Quad((
      self.layout.into_quad(),
      Style::SolidColor((0., 0., 0., 0.2).into()),
    )));
  }
}

impl LayoutAble for Text {
  fn layout(&mut self, constraint: LayoutConstraint, _ctx: &mut LayoutCtx) -> LayoutResult {
    if self.layout.skipable(constraint) {
      return self.layout.size.with_default_baseline();
    }

    self.layout.size = constraint.max();
    self.layout.size.with_default_baseline()
  }

  fn set_position(&mut self, position: UIPosition) {
    self.layout.set_relative_position(position)
  }
}
