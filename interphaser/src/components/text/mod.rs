use ab_glyph::*;
use glyph_brush::*;

use crate::text::LineWrap;
use crate::text::TextInfo;
use crate::*;

pub use glyph_brush::HorizontalAlign;
pub use glyph_brush::VerticalAlign;

mod cursor;
pub use cursor::*;

mod editable;
pub use editable::*;

pub(crate) type TextLayout = Vec<SectionGlyph>;

pub struct Text {
  pub content: LayoutSource<String>,
  pub line_wrap: LineWrap,
  pub horizon_align: HorizontalAlign,
  pub vertical_align: VerticalAlign,
  pub text_layout: Option<TextLayout>,
  pub layout: LayoutUnit,
}

impl Default for Text {
  fn default() -> Self {
    Self {
      content: LayoutSource::new("".into()),
      layout: Default::default(),
      horizon_align: HorizontalAlign::Center,
      vertical_align: VerticalAlign::Center,
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

  pub fn with_horizon_align(mut self, horizon_align: HorizontalAlign) -> Self {
    self.horizon_align = horizon_align;
    self
  }

  pub fn with_vertical_align(mut self, vertical_align: VerticalAlign) -> Self {
    self.vertical_align = vertical_align;
    self
  }

  pub(crate) fn get_text_layout(&mut self, fonts: &FontManager) -> &Vec<SectionGlyph> {
    self.text_layout.get_or_insert_with(|| {
      let x_correct = match self.horizon_align {
        glyph_brush::HorizontalAlign::Left => 0.,
        glyph_brush::HorizontalAlign::Center => self.layout.size.width / 2.,
        glyph_brush::HorizontalAlign::Right => self.layout.size.width,
      };

      let y_correct = match self.vertical_align {
        glyph_brush::VerticalAlign::Top => 0.,
        glyph_brush::VerticalAlign::Center => self.layout.size.height / 2.,
        glyph_brush::VerticalAlign::Bottom => self.layout.size.height / 2.,
      };

      let layout = Layout::SingleLine {
        line_breaker: BuiltInLineBreaker::default(),
        h_align: HorizontalAlign::Center,
        v_align: VerticalAlign::Center,
      };
      let geometry = SectionGeometry {
        screen_position: (
          self.layout.absolute_position.x + x_correct,
          self.layout.absolute_position.y + y_correct,
        ),
        bounds: self.layout.size.into(),
      };

      layout.calculate_glyphs(
        fonts.get_fonts().as_slice(),
        &geometry,
        &[SectionText {
          text: self.content.get().as_str(),
          scale: PxScale::from(30.0),
          font_id: FontId(0),
        }],
      )
    })
  }
}

impl<T> Component<T> for Text {
  fn update(&mut self, _: &T, ctx: &mut UpdateCtx) {
    self.layout.check_attach(ctx);
    self.content.refresh(&mut self.layout, ctx);
  }
}

impl Presentable for Text {
  fn render(&mut self, builder: &mut PresentationBuilder) {
    self.layout.update_world(builder.current_origin_offset);

    builder.present.primitives.push(Primitive::Text(TextInfo {
      content: self.content.get().clone(),
      bounds: self.layout.size,
      line_wrap: self.line_wrap,
      horizon_align: self.horizon_align,
      vertical_align: self.vertical_align,
      x: self.layout.absolute_position.x,
      y: self.layout.absolute_position.y,
      color: (0., 0., 0., 1.).into(),
      font_size: 30.,
    }));

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
