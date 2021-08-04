use crate::*;

#[derive(Default)]
pub struct Memo<T> {
  cached: Option<T>,
}

impl<T: PartialEq + Clone, C: Component<T>> ComponentAbility<T, C> for Memo<T> {
  fn update(&mut self, model: &T, inner: &mut C, ctx: &mut UpdateCtx) {
    if let Some(cached) = self.cached.as_ref() {
      if cached == model {
        return;
      }
    }
    inner.update(model, ctx);
    self.cached = model.clone().into();
  }
}

impl<T, C: Presentable> PresentableAbility<C> for Memo<T> {
  fn render(&self, builder: &mut PresentationBuilder, inner: &C) {
    inner.render(builder);
  }
}

impl<T, C: LayoutAble> LayoutAbility<C> for Memo<T> {
  fn layout(
    &mut self,
    constraint: LayoutConstraint,
    ctx: &mut LayoutCtx,
    inner: &mut C,
  ) -> LayoutResult {
    inner.layout(constraint, ctx)
  }

  fn set_position(&mut self, position: UIPosition, inner: &mut C) {
    inner.set_position(position)
  }
}

impl<T, C: HotAreaProvider> HotAreaPassBehavior<C> for Memo<T> {
  fn is_point_in(&self, point: crate::UIPosition, inner: &C) -> bool {
    inner.is_point_in(point)
  }
}
