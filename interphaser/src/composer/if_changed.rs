use crate::*;

#[derive(Default)]
pub struct IfChanged<T> {
  cached: Option<T>,
  sub_tree_has_update: bool,
}

impl<T: PartialEq + Clone, C: Component<T>> ComponentAbility<T, C> for IfChanged<T> {
  fn update(&mut self, model: &T, inner: &mut C, ctx: &mut UpdateCtx) {
    if !self.sub_tree_has_update {
      if let Some(cached) = self.cached.as_ref() {
        if cached == model {
          return;
        } else {
          self.cached = model.clone().into();
        }
      }
    }

    inner.update(model, ctx);
  }
  fn event(&mut self, model: &mut T, event: &mut EventCtx, inner: &mut C) {
    inner.event(model, event);
    self.sub_tree_has_update |= event.view_may_changed
  }
}

impl<T, C: Presentable> PresentableAbility<C> for IfChanged<T> {
  fn render(&mut self, builder: &mut PresentationBuilder, inner: &mut C) {
    inner.render(builder);
  }
}

impl<T, C: LayoutAble> LayoutAbility<C> for IfChanged<T> {
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

impl<T, C: HotAreaProvider> HotAreaPassBehavior<C> for IfChanged<T> {
  fn is_point_in(&self, point: crate::UIPosition, inner: &C) -> bool {
    inner.is_point_in(point)
  }
}
