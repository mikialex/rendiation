use rendiation_algebra::Vec4;

use crate::*;

#[derive(Default)]
pub struct LayoutSource<T> {
  value: T,
  changed: bool,
}

impl<T> LayoutSource<T> {
  pub fn new(value: T) -> Self {
    Self {
      value,
      changed: true,
    }
  }
  pub fn set(&mut self, value: impl Into<T>) {
    self.value = value.into();
    self.changed = true;
  }

  pub fn get(&mut self) -> &T {
    &self.value
  }

  pub fn refresh(&mut self, layout: &mut LayoutUnit, ctx: &mut UpdateCtx) {
    if self.changed {
      layout.request_layout(ctx)
    }
    self.changed = false;
  }
}

pub struct Container {
  pub size: LayoutSource<LayoutSize>,
  pub color: Vec4<f32>,
  layout: LayoutUnit,
}

impl Container {
  pub fn size(size: impl Into<LayoutSize>) -> Self {
    Self {
      size: LayoutSource::new(size.into()),
      color: Vec4::new(1., 1., 1., 0.),
      layout: Default::default(),
    }
  }
}

impl<T> Component<T> for Container {
  fn update(&mut self, _model: &T, ctx: &mut UpdateCtx) {
    self.layout.check_attach(ctx); // this is useless todo
    self.size.refresh(&mut self.layout, ctx);
  }
}

impl<T, C: Component<T>> ComponentAbility<T, C> for Container {
  fn update(&mut self, model: &T, inner: &mut C, ctx: &mut UpdateCtx) {
    self.layout.check_attach(ctx); // this is useless todo
    self.size.refresh(&mut self.layout, ctx);
    inner.update(model, ctx);
    self.layout.or_layout_change(ctx);
  }

  fn event(&mut self, model: &mut T, event: &mut EventCtx, inner: &mut C) {
    inner.event(model, event);
  }
}

impl<C: Presentable> PresentableAbility<C> for Container {
  fn render(&mut self, builder: &mut PresentationBuilder, inner: &mut C) {
    self.layout.update_world(builder.current_origin_offset);
    builder.present.primitives.push(Primitive::Quad((
      self.layout.into_quad(),
      Style::SolidColor(self.color),
    )));
    builder.push_offset(self.layout.relative_position);
    inner.render(builder);
    builder.pop_offset()
  }
}

impl<C: LayoutAble> LayoutAbility<C> for Container {
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

  fn set_position(&mut self, position: UIPosition, _inner: &mut C) {
    self.layout.set_relative_position(position);
  }
}

impl<C> HotAreaPassBehavior<C> for Container {
  fn is_point_in(&self, point: crate::UIPosition, _inner: &C) -> bool {
    self.layout.into_quad().is_point_in(point)
  }
}
