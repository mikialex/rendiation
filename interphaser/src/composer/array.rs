use crate::*;

#[derive(Default)]
pub struct ComponentArray<C> {
  pub children: Vec<C>,
}

impl<C> From<Vec<C>> for ComponentArray<C> {
  fn from(children: Vec<C>) -> Self {
    Self { children }
  }
}

impl<X> ComponentArray<X> {
  #[must_use]
  pub fn child(mut self, x: X) -> Self {
    self.children.push(x);
    self
  }
}

type IterType<'a, C: 'a> = impl Iterator<Item = &'a mut C> + 'a + ExactSizeIterator;

impl<'a, C> IntoIterator for &'a mut ComponentArray<C> {
  type Item = &'a mut C;
  type IntoIter = IterType<'a, C>;

  fn into_iter(self) -> IterType<'a, C> {
    self.children.iter_mut()
  }
}

impl<C: Presentable> Presentable for ComponentArray<C> {
  fn render(&mut self, builder: &mut PresentationBuilder) {
    self.children.iter_mut().for_each(|c| c.render(builder))
  }
}

impl<C> Eventable for ComponentArray<C>
where
  C: Eventable,
{
  fn event(&mut self, event: &mut crate::EventCtx) {
    self.children.iter_mut().for_each(|c| c.event(event))
  }
}

impl<C> Stream for ComponentArray<C>
where
  C: Stream<Item = ()> + Unpin,
{
  type Item = ();

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let mut view_changed = false;
    for child in &mut self.children {
      view_changed |= child.poll_next_unpin(cx).is_ready();
    }
    if view_changed {
      Poll::Ready(().into())
    } else {
      Poll::Pending
    }
  }
}
