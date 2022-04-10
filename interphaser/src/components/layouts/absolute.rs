use crate::*;

#[derive(Default)]
pub struct AbsoluteAnchor {
  position: UIPosition,
}

impl<T, C: Component<T>> ComponentAbility<T, C> for AbsoluteAnchor {
  fn update(&mut self, model: &T, inner: &mut C, ctx: &mut UpdateCtx) {
    inner.update(model, ctx);
  }
  fn event(&mut self, model: &mut T, event: &mut EventCtx, inner: &mut C) {
    inner.event(model, event);
  }
}

impl<C: Presentable> PresentableAbility<C> for AbsoluteAnchor {
  fn render(&mut self, builder: &mut PresentationBuilder, inner: &mut C) {
    builder.push_offset(self.position);
    inner.render(builder);
    builder.pop_offset()
  }
}

impl<C: HotAreaProvider> HotAreaPassBehavior<C> for AbsoluteAnchor {
  fn is_point_in(&self, point: crate::UIPosition, inner: &C) -> bool {
    inner.is_point_in(point)
  }
}

pub fn absolute_group<T>() -> ComponentArray<AbsChild<T>> {
  ComponentArray {
    children: Vec::new(),
  }
}

pub struct AbsChild<T> {
  pub position: UIPosition,
  pub inner: Box<dyn UIComponent<T>>,
}

impl<T> AbsChild<T> {
  pub fn new(inner: impl UIComponent<T> + 'static) -> Self {
    Self {
      inner: Box::new(inner),
      position: Default::default(),
    }
  }

  #[must_use]
  pub fn with_position(mut self, position: impl Into<UIPosition>) -> Self {
    self.position = position.into();
    self
  }
}

impl<T> Component<T> for AbsChild<T> {
  fn event(&mut self, model: &mut T, event: &mut EventCtx) {
    self.inner.event(model, event)
  }

  fn update(&mut self, model: &T, ctx: &mut UpdateCtx) {
    self.inner.update(model, ctx)
  }
}

impl<T> Presentable for AbsChild<T> {
  fn render(&mut self, builder: &mut PresentationBuilder) {
    self.inner.render(builder)
  }
}

impl<T, C> LayoutAbility<C> for AbsoluteAnchor
where
  for<'a> &'a mut C: IntoIterator<Item = &'a mut AbsChild<T>, IntoIter: ExactSizeIterator>,
{
  fn layout(
    &mut self,
    constraint: LayoutConstraint,
    ctx: &mut LayoutCtx,
    inner: &mut C,
  ) -> LayoutResult {
    inner.into_iter().for_each(|child| {
      child.inner.layout(constraint, ctx);
    });

    constraint.max().with_default_baseline()
  }

  fn set_position(&mut self, position: UIPosition, inner: &mut C) {
    self.position = position;
    inner.into_iter().for_each(|child| {
      child.inner.set_position(child.position);
    });
  }
}
