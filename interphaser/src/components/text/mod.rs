use crate::*;

mod cursor;
pub use cursor::*;

mod editable;
pub use editable::*;

pub enum TextLayoutConfig {
  /// The layout will use parent max box constraint as bound box
  SizedBox {
    line_wrap: LineWrap,
    horizon_align: TextHorizontalAlignment,
    vertical_align: TextVerticalAlignment,
  },
  SingleLineShrink,
}

impl Default for TextLayoutConfig {
  fn default() -> Self {
    Self::SizedBox {
      line_wrap: Default::default(),
      horizon_align: Default::default(),
      vertical_align: Default::default(),
    }
  }
}

pub struct Text {
  pub content: String,
  pub layout_config: TextLayoutConfig,
  pub text_layout_cache: Option<TextLayoutRef>,
  pub text_layout_size_cache: Option<UISize>,
  pub layout: LayoutUnit,
}

impl Eventable for Text {
  fn event(&mut self, event: &mut EventCtx) {}
}

impl Default for Text {
  fn default() -> Self {
    Self {
      content: "".into(),
      layout: Default::default(),
      layout_config: Default::default(),
      text_layout_cache: None,
      text_layout_size_cache: None,
    }
  }
}

impl Text {
  pub fn new(content: impl Into<String>) -> Self {
    Self {
      content: content.into(),
      ..Default::default()
    }
  }

  // todo, put it in setters
  pub fn reset_text_layout_cache(&mut self) {
    self.text_layout_cache = None;
    self.text_layout_size_cache = None;
  }

  #[must_use]
  pub fn with_layout(mut self, config: TextLayoutConfig) -> Self {
    self.layout_config = config;
    self
  }

  pub(super) fn get_text_layout(
    &mut self,
    fonts: &FontManager,
    text: &mut TextCache,
  ) -> &TextLayoutRef {
    self.text_layout_cache.get_or_insert_with(|| {
      let text_info = match self.layout_config {
        TextLayoutConfig::SizedBox {
          line_wrap,
          horizon_align,
          vertical_align,
        } => TextInfo {
          content: self.content.clone(),
          bounds: self.layout.size.into(),
          line_wrap,
          horizon_align,
          vertical_align,
          x: self.layout.absolute_position.x,
          y: self.layout.absolute_position.y,
          color: (0., 0., 0., 1.).into(),
          font_size: 30.,
        },
        TextLayoutConfig::SingleLineShrink => TextInfo {
          content: self.content.clone(),
          bounds: self.layout.size.into(),
          line_wrap: LineWrap::Single,
          horizon_align: TextHorizontalAlignment::Left,
          vertical_align: TextVerticalAlignment::Center,
          x: self.layout.absolute_position.x,
          y: self.layout.absolute_position.y,
          color: (0., 0., 0., 1.).into(),
          font_size: 30.,
        },
      };

      text.cache_layout(&text_info, fonts)
    })
  }

  pub fn get_text_boundary(&mut self, fonts: &FontManager, text: &TextCache) -> &UISize {
    self.text_layout_size_cache.get_or_insert_with(|| {
      text
        .measure_size(
          &TextRelaxedInfo {
            content: self.content.clone(),
            font_size: 30.,
          },
          fonts,
        )
        .into()
    })
  }
}

impl Presentable for Text {
  fn render(&mut self, builder: &mut PresentationBuilder) {
    self.layout.update_world(builder.current_origin_offset());

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
  fn layout(&mut self, constraint: LayoutConstraint, ctx: &mut LayoutCtx) -> LayoutResult {
    match self.layout_config {
      TextLayoutConfig::SingleLineShrink => {
        let size = self.get_text_boundary(ctx.fonts, ctx.text);
        self.layout.size = constraint.clamp(*size);
      }
      _ => {
        self.layout.size = constraint.max();
      }
    }

    self.layout.size.with_default_baseline()
  }

  fn set_position(&mut self, position: UIPosition) {
    self.layout.set_relative_position(position)
  }
}
