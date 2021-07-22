use rendiation_algebra::Vec4;

use crate::*;

pub struct Container<T> {
  pub size: Value<LayoutSize, T>,
  pub color: Vec4<f32>,
  self_position_computed: UIPosition,
  child_position_relative: UIPosition,
  size_computed: LayoutSize,
  quad_cache: Quad,
}

impl<T> Container<T> {
  pub fn size(size: LayoutSize) -> Self {
    Self {
      size: Value::Static(size),
      color: Vec4::new(1., 1., 1., 0.),
      self_position_computed: Default::default(),
      child_position_relative: Default::default(),
      size_computed: Default::default(),
      quad_cache: Default::default(),
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

impl<T, C: Presentable> PresentableAbility<C> for Container<T> {
  fn render(&self, builder: &mut PresentationBuilder, inner: &C) {
    builder
      .present
      .primitives
      .push(Primitive::Quad(self.quad_cache));
    inner.render(builder);
  }
}

impl<T, C: LayoutAble> LayoutAbility<C> for Container<T> {
  fn layout(&mut self, constraint: LayoutConstraint, inner: &mut C) -> LayoutSize {
    let child_size = inner.layout(constraint);
    self.size_computed = constraint.clamp(*self.size.get());
    self.child_position_relative = UIPosition { x: 0., y: 0. };
    self.quad_cache.width = self.size_computed.width;
    self.quad_cache.height = self.size_computed.height;
    self.size_computed
  }

  fn set_position(&mut self, position: UIPosition, inner: &mut C) {
    self.self_position_computed = position;
    self.quad_cache.x = position.x;
    self.quad_cache.y = position.y;

    inner.set_position(UIPosition {
      x: position.x + self.child_position_relative.x,
      y: position.y + self.child_position_relative.y,
    })
  }
}

impl<T, C> HotAreaPassBehavior<C> for Container<T> {
  fn is_point_in(&self, point: crate::UIPosition, inner: &C) -> bool {
    self.quad_cache.is_point_in(point)
  }
}
