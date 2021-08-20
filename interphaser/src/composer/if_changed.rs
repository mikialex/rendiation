use crate::*;

#[derive(Default)]
pub struct IfChanged<T> {
  cached: Option<T>,
  extra_states: Vec<(Box<dyn VersionedValue>, u32)>,
}

impl<T> IfChanged<T> {
  pub fn with_state<S: 'static>(mut self, s: &StateCell<VersionedCell<S>>) -> Self {
    let s = s.boxed();
    let version = s.get_version();
    self.extra_states.push((s, version));
    self
  }

  fn is_state_changed(&mut self) -> bool {
    let mut has_change = false;
    for (state, old_version) in &mut self.extra_states {
      let new_version = state.get_version();
      if new_version != *old_version {
        has_change = true;
        *old_version = new_version;
      }
    }
    has_change
  }
}

impl<T: PartialEq + Clone, C: Component<T>> ComponentAbility<T, C> for IfChanged<T> {
  fn update(&mut self, model: &T, inner: &mut C, ctx: &mut UpdateCtx) {
    if !self.is_state_changed() {
      if let Some(cached) = self.cached.as_ref() {
        if cached == model {
          return;
        }
      }
    }
    inner.update(model, ctx);
    self.cached = model.clone().into();
  }
  fn event(&mut self, model: &mut T, event: &mut EventCtx, inner: &mut C) {
    inner.event(model, event);
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
