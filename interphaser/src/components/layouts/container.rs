use rendiation_algebra::Vec4;

use crate::*;

pub struct Container<T> {
  pub size: Value<LayoutSize, T>,
  pub color: Value<Vec4<f32>, T>,
  child_position_relative: UIPosition,
  layout: LayoutUnit,
}

impl<T> Container<T> {
  pub fn size(size: LayoutSize) -> Self {
    Self {
      size: Value::Static(size),
      color: Value::Static(Vec4::new(1., 1., 1., 0.)),
      child_position_relative: Default::default(),
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
    self.size.update(model);
    self.color.update(model);
    inner.update(model, ctx);
  }

  fn event(&mut self, model: &mut T, event: &mut EventCtx, inner: &mut C) {
    inner.event(model, event);
  }
}

impl<T, C: Presentable> PresentableAbility<C> for Container<T> {
  fn render(&self, builder: &mut PresentationBuilder, inner: &C) {
    builder.present.primitives.push(Primitive::Quad((
      self.layout.into_quad(),
      Style::SolidColor(*self.color.get()),
    )));
    inner.render(builder);
  }
}

impl<T, C: LayoutAble> LayoutAbility<C> for Container<T> {
  fn layout(
    &mut self,
    constraint: LayoutConstraint,
    ctx: &mut LayoutCtx,
    inner: &mut C,
  ) -> LayoutSize {
    let child_size = inner.layout(constraint, ctx);
    self.layout.size = constraint.clamp(*self.size.get());
    self.child_position_relative = UIPosition { x: 0., y: 0. };
    self.layout.size
  }

  fn set_position(&mut self, position: UIPosition, inner: &mut C) {
    self.layout.position = position;

    inner.set_position(UIPosition {
      x: position.x + self.child_position_relative.x,
      y: position.y + self.child_position_relative.y,
    })
  }
}

impl<T, C> HotAreaPassBehavior<C> for Container<T> {
  fn is_point_in(&self, point: crate::UIPosition, inner: &C) -> bool {
    self.layout.into_quad().is_point_in(point)
  }
}
