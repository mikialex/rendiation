use rendiation_algebra::Vec4;

use crate::*;

pub struct Container<T> {
  pub size: Value<LayoutSize, T>,
  pub color: Vec4<f32>,
  position_computed: UIPosition,
  size_computed: LayoutSize,
}

impl<T> Container<T> {
  fn size(size: LayoutSize) -> Self {
    Self {
      size: Value::Static(size),
      color: Vec4::new(1., 1., 1., 0.),
      position_computed: Default::default(),
      size_computed: Default::default(),
    }
  }
}

impl<T, C: Component<T>> ComponentAbility<T, C> for Container<T> {
  fn update(&mut self, model: &T, inner: &mut C, ctx: &mut UpdateCtx) {
    self.size.update(model);
    inner.update(model, ctx);
  }

  fn event(&mut self, model: &mut T, event: &mut EventCtx, inner: &mut C) {
    inner.event(model, event);
  }
}

impl<T> Presentable for Container<T> {
  fn render(&self, builder: &mut PresentationBuilder) {
    builder.present.primitives.push(Primitive::Quad(Quad {
      x: self.position_computed.x,
      y: self.position_computed.y,
      width: self.size_computed.width,
      height: self.size_computed.height,
    }));
  }
}

impl<T> LayoutAble<T> for Container<T> {
  fn layout(&mut self, constraint: LayoutConstraint) -> LayoutSize {
    self.size_computed = constraint.clamp(*self.size.get());
    self.size_computed
  }

  fn set_position(&mut self, position: UIPosition) {
    self.position_computed = position;
  }
}

impl<T, C> HotAreaPassBehavior<C> for Container<T> {
  fn is_point_in(&self, point: crate::UIPosition, inner: &C) -> bool {
    // inner.is_point_in(point)
    todo!()
  }
}
