use ab_glyph::*;
use glyph_brush::*;

use crate::*;

pub use glyph_brush::HorizontalAlign;
pub use glyph_brush::VerticalAlign;

pub struct Text {
  pub content: LayoutSource<String>,
  pub line_wrap: LineWrap,
  pub horizon_align: HorizontalAlign,
  pub vertical_align: VerticalAlign,
  text_layout: Option<Vec<SectionGlyph>>,
  layout: LayoutUnit,
}

impl Text {
  pub fn new(content: impl Into<String>) -> Self {
    Self {
      content: LayoutSource::new(content.into()),
      layout: Default::default(),
      horizon_align: HorizontalAlign::Center,
      vertical_align: VerticalAlign::Center,
      line_wrap: Default::default(),
      text_layout: None,
    }
  }

  pub fn get_text_layout(&mut self, fonts: &FontManager) -> &Vec<SectionGlyph> {
    self.text_layout.get_or_insert_with(|| {
      let layout = Layout::SingleLine {
        line_breaker: BuiltInLineBreaker::default(),
        h_align: HorizontalAlign::Center,
        v_align: VerticalAlign::Center,
      };
      let geometry = SectionGeometry::default();

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
      x: self.layout.position.x,
      y: self.layout.position.y,
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
