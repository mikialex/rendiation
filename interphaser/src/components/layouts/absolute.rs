use crate::*;

#[derive(Default)]
pub struct AbsoluteAnchor {
  position: UIPosition,
}

impl<C: Eventable> ComponentAbility<C> for AbsoluteAnchor {
  fn event(&mut self, event: &mut EventCtx, inner: &mut C) {
    inner.event(event);
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

pub fn absolute_group() -> Vec<AbsChild> {
  Vec::new()
}

pub struct AbsChild {
  pub position: UIPosition,
  pub inner: Box<dyn Component>,
}

impl AbsChild {
  pub fn new(inner: impl Component + 'static) -> Self {
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

impl Eventable for AbsChild {
  fn event(&mut self, event: &mut EventCtx) {
    self.inner.event(event)
  }
}

impl Presentable for AbsChild {
  fn render(&mut self, builder: &mut PresentationBuilder) {
    self.inner.render(builder)
  }
}

impl<C> LayoutAbility<C> for AbsoluteAnchor
where
  for<'a> &'a mut C: IntoIterator<Item = &'a mut AbsChild, IntoIter: ExactSizeIterator>,
{
  fn layout(
    &mut self,
    constraint: LayoutConstraint,
    ctx: &mut LayoutCtx,
    inner: &mut C,
  ) -> LayoutResult {
    // we just pass the parent constraint to children, so the anchor itself is
    // transparent to children
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
