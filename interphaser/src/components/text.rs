use rendiation_algebra::Vec4;

use crate::*;

pub struct Text<T> {
  content: Value<String, T>,
  position_computed: UIPosition,
  size_computed: Option<LayoutSize>,
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
    if self.content.update_and_check_changed(model).1 {
      self.size_computed = None;
    }
  }
}

impl<T> Presentable for Text<T> {
  fn render(&self, builder: &mut PresentationBuilder) {
    builder.present.primitives.push(Primitive::Text(TextInfo {
      content: self.content.get().clone(),
      max_width: Some(100.),
      x: self.position_computed.x,
      y: self.position_computed.y,
      color: Vec4::new(0., 0., 0., 1.),
      font_size: 30.,
    }));
  }
}

impl<T> LayoutAble for Text<T> {
  fn layout(&mut self, constraint: LayoutConstraint, ctx: &mut LayoutCtx) -> LayoutSize {
    use glyph_brush::{ab_glyph::*, *};
    *self.size_computed.get_or_insert_with(|| {
      let glyphs = Layout::default().calculate_glyphs(
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
      LayoutSize {
        width: max_width,
        height: max_height,
      }
    })
  }

  fn set_position(&mut self, position: UIPosition) {
    self.position_computed = position;
  }
}
