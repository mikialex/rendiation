use crate::*;

pub struct AbsoluteAnchor {
  position: UIPosition,
}

pub struct AbsolutePositionChild<T> {
  pub position: UIPosition,
  pub inner: Box<dyn UIComponent<T>>,
}

impl<T, C> LayoutAbility<C> for AbsoluteAnchor
where
  for<'a> &'a mut C:
    IntoIterator<Item = &'a mut AbsolutePositionChild<T>, IntoIter: ExactSizeIterator2<'a>>,
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
