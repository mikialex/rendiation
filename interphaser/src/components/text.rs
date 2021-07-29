use rendiation_algebra::Vec4;

use crate::*;

pub struct Text<T> {
  content: Value<String, T>,
  layout: LayoutUnit,
  layout_size_dirty: bool,
}

impl<T> Text<T> {
  pub fn new(content: impl Into<Value<String, T>>) -> Self {
    Self {
      content: content.into(),
      layout: Default::default(),
      layout_size_dirty: true,
    }
  }
}

impl<T> Component<T> for Text<T> {
  fn update(&mut self, model: &T, ctx: &mut UpdateCtx) {
    if self.content.update_and_check_changed(model).1 {
      self.layout_size_dirty = true;
    }
  }
}

impl<T> Presentable for Text<T> {
  fn render(&self, builder: &mut PresentationBuilder) {
    builder.present.primitives.push(Primitive::Text(TextInfo {
      content: self.content.get().clone(),
      max_width: Some(100.),
      x: self.layout.position.x,
      y: self.layout.position.y,
      color: Vec4::new(0., 0., 0., 1.),
      font_size: 30.,
    }));

    builder.present.primitives.push(Primitive::Quad((
      self.layout.into_quad(),
      Style::SolidColor(Vec4::new(0., 0., 0., 0.2)),
    )));
  }
}

impl<T> LayoutAble for Text<T> {
  fn layout(&mut self, constraint: LayoutConstraint, ctx: &mut LayoutCtx) -> LayoutSize {
    use glyph_brush::{ab_glyph::*, *};
    if self.layout_size_dirty {
      let glyphs = Layout::SingleLine {
        line_breaker: BuiltInLineBreaker::default(),
        h_align: HorizontalAlign::Center,
        v_align: VerticalAlign::Center,
      }
      .calculate_glyphs(
        ctx.fonts.get_fonts().as_slice(),
        &SectionGeometry::default(),
        &[SectionText {
          text: self.content.get().as_str(),
          scale: PxScale::from(30.0),
          font_id: FontId(0),
        }],
      );
      let mut max_width = 0.0_f32;
      let mut max_height = 0.0_f32;
      glyphs.iter().for_each(|glyph| {
        max_width = max_width.max(glyph.glyph.position.x + glyph.glyph.scale.x);
        max_height = max_height.max(glyph.glyph.position.y + glyph.glyph.scale.y);
      });
      println!("{}, {}", max_width, max_height);

      self.layout.size = LayoutSize {
        width: max_width,
        height: max_height,
      };
      self.layout_size_dirty = false;
    }

    self.layout.size
  }

  fn set_position(&mut self, position: UIPosition) {
    self.layout.position = position;
  }
}
