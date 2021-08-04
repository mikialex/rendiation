use rendiation_algebra::Vec4;

use crate::*;

pub struct Container<T> {
  pub size: Value<LayoutSize, T>,
  pub color: Value<Vec4<f32>, T>,
  layout: LayoutUnit,
}

impl<T> Container<T> {
  pub fn size(size: LayoutSize) -> Self {
    Self {
      size: Value::Static(size),
      color: Value::Static(Vec4::new(1., 1., 1., 0.)),
      layout: Default::default(),
    }
  }
  pub fn color(mut self, color: impl Into<Value<Vec4<f32>, T>>) -> Self {
    self.color = color.into();
    self
  }
}

impl<T, C: Component<T>> ComponentAbility<T, C> for Container<T> {
  fn update(&mut self, model: &T, inner: &mut C, ctx: &mut UpdateCtx) {
    self.layout.check_attach(ctx);

    if self.size.diff_update(model).changed {
      self.layout.request_layout(ctx)
    }
    self.color.update(model);
    inner.update(model, ctx);
    self.layout.or_layout_change(ctx);
  }

  fn event(&mut self, model: &mut T, event: &mut EventCtx, inner: &mut C) {
    inner.event(model, event);
  }
}

impl<T, C: Presentable> PresentableAbility<C> for Container<T> {
  fn render(&mut self, builder: &mut PresentationBuilder, inner: &mut C) {
    self.layout.update_world(builder.current_origin_offset);
    builder.present.primitives.push(Primitive::Quad((
      self.layout.into_quad(),
      Style::SolidColor(*self.color.get()),
    )));
    builder.push_offset(self.layout.relative_position);
    inner.render(builder);
    builder.pop_offset()
  }
}

impl<T, C: LayoutAble> LayoutAbility<C> for Container<T> {
  fn layout(
    &mut self,
    constraint: LayoutConstraint,
    ctx: &mut LayoutCtx,
    inner: &mut C,
  ) -> LayoutResult {
    if self.layout.skipable(constraint) {
      return self.layout.size.with_default_baseline();
    }
    let child_size = inner.layout(constraint, ctx).size;
    self.layout.size = constraint.clamp(*self.size.get());

    let child_offset_x = self.layout.size.width - child_size.width;
    let child_offset_x = child_offset_x.max(0.) * 0.5;
    let child_offset_y = self.layout.size.height - child_size.height;
    let child_offset_y = child_offset_y.max(0.) * 0.5;

    inner.set_position(UIPosition {
      x: child_offset_x,
      y: child_offset_y,
    });

    self.layout.size.with_default_baseline()
  }

  fn set_position(&mut self, position: UIPosition, inner: &mut C) {
    self.layout.set_relative_position(position);
  }
}

impl<T, C> HotAreaPassBehavior<C> for Container<T> {
  fn is_point_in(&self, point: crate::UIPosition, inner: &C) -> bool {
    self.layout.into_quad().is_point_in(point)
  }
}
