use rendiation_algebra::Vec4;

use crate::*;

pub struct Text<T> {
  content: Value<String, T>,
  layout: LayoutUnit,
}

impl<T> Text<T> {
  pub fn new(content: impl Into<Value<String, T>>) -> Self {
    Self {
      content: content.into(),
      layout: Default::default(),
    }
  }
}

impl<T> Component<T> for Text<T> {
  fn update(&mut self, model: &T, ctx: &mut UpdateCtx) {
    if self.content.diff_update(model).changed {
      self.layout.request_layout(ctx);
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
    if self.layout.skipable(constraint) {
      return self.layout.size;
    }

    use glyph_brush::{ab_glyph::*, *};
    let layout = Layout::SingleLine {
      line_breaker: BuiltInLineBreaker::default(),
      h_align: HorizontalAlign::Center,
      v_align: VerticalAlign::Center,
    };
    let geometry = SectionGeometry::default();

    let size = layout
      .calculate_glyphs(
        ctx.fonts.get_fonts().as_slice(),
        &geometry,
        &[SectionText {
          text: self.content.get().as_str(),
          scale: PxScale::from(30.0),
          font_id: FontId(0),
        }],
      )
      .iter()
      .fold(None, |b: Option<Rect>, sg| {
        let bounds = ctx.fonts.get_font(sg.font_id).glyph_bounds(&sg.glyph);
        b.map(|b| {
          let min_x = b.min.x.min(bounds.min.x);
          let max_x = b.max.x.max(bounds.max.x);
          let min_y = b.min.y.min(bounds.min.y);
          let max_y = b.max.y.max(bounds.max.y);
          Rect {
            min: point(min_x, min_y),
            max: point(max_x, max_y),
          }
        })
        .or(Some(bounds))
      })
      .map(|mut b| {
        // cap the glyph bounds to the layout specified max bounds
        let Rect { min, max } = layout.bounds_rect(&geometry);
        b.min.x = b.min.x.max(min.x);
        b.min.y = b.min.y.max(min.y);
        b.max.x = b.max.x.min(max.x);
        b.max.y = b.max.y.min(max.y);
        b
      })
      .unwrap_or(Rect::default());

    let max_width = size.max.x - size.min.x;
    let max_height = size.max.y - size.min.y;

    self.layout.size = LayoutSize {
      width: max_width,
      height: max_height,
    };

    self.layout.size
  }

  fn set_position(&mut self, position: UIPosition) {
    self.layout.position = position;
  }
}
