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

impl<'a, C> IntoIterator for &'a mut ComponentArray<C> {
  type Item = &'a mut C;
  type IntoIter = impl Iterator<Item = &'a mut C> + 'a + ExactSizeIterator;

  fn into_iter(self) -> Self::IntoIter {
    self.children.iter_mut()
  }
}

impl<C: View> View for ComponentArray<C> {
  fn request(&mut self, detail: &mut ViewRequest) {
    match detail {
      // todo, union, not triggered because now covered by layouter
      ViewRequest::Layout(_) => todo!(),
      ViewRequest::HitTest { point, result } => {
        **result = self.into_iter().any(|child| child.hit_test(*point));
      }
      _ => self.children.iter_mut().for_each(|c| c.request(detail)),
    }
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
