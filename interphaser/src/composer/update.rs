use std::ops::DerefMut;

use crate::*;

pub struct ReactiveNestedView<C, X> {
  pub updater: X,
  pub inner: C,
}

impl<C, X> AsMut<C> for ReactiveNestedView<C, X> {
  fn as_mut(&mut self) -> &mut C {
    &mut self.inner
  }
}

pub trait ReactiveUpdateNester<C>: Unpin {
  fn poll_update_inner(
    self: Pin<&mut Self>,
    cx: &mut Context<'_>,
    inner: &mut C,
  ) -> Poll<Option<()>>;
}

pub trait ViewReactExt: Sized {
  fn react<X>(self, updater: X) -> ReactiveNestedView<Self, X>
  where
    X: ReactiveUpdateNester<Self>,
  {
    ReactiveNestedView {
      updater,
      inner: self,
    }
  }
}
impl<X> ViewReactExt for X where X: View {}

impl<C, X> Stream for ReactiveNestedView<C, X>
where
  X: ReactiveUpdateNester<C> + Unpin,
  C: View,
  Self: Unpin,
{
  type Item = ();

  fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.deref_mut();
    Pin::new(&mut this.updater).poll_update_inner(cx, &mut this.inner)
  }
}
impl<C: View, X> View for ReactiveNestedView<C, X>
where
  Self: Stream<Item = ()> + Unpin,
{
  fn request(&mut self, detail: &mut ViewRequest) {
    self.inner.request(detail)
  }
}

/// when ReactiveNestedView act as view nest, the ReactiveNestedView's content: inner should impl
/// ViewNester over the nested view
impl<C: ViewNester<CC>, X, CC> ViewNester<CC> for ReactiveNestedView<C, X> {
  fn request_nester(&mut self, detail: &mut ViewRequest, inner: &mut CC) {
    self.inner.request_nester(detail, inner)
  }
}
pub struct ReactiveUpdaterGroup<C> {
  updater: Vec<Box<dyn ReactiveUpdateNester<C>>>,
}

impl<C> Default for ReactiveUpdaterGroup<C> {
  fn default() -> Self {
    Self {
      updater: Default::default(),
    }
  }
}

impl<C> ReactiveUpdaterGroup<C> {
  pub fn with(mut self, another: impl ReactiveUpdateNester<C> + 'static) -> Self {
    self.updater.push(Box::new(another));
    self
  }
}

impl<C> ReactiveUpdateNester<C> for ReactiveUpdaterGroup<C>
where
  C: Stream<Item = ()> + Unpin,
{
  fn poll_update_inner(
    mut self: Pin<&mut Self>,
    cx: &mut Context<'_>,
    inner: &mut C,
  ) -> Poll<Option<()>> {
    let mut r = false;
    let this = self.deref_mut();
    for update in &mut this.updater {
      r |= Pin::new(update.as_mut())
        .poll_update_inner(cx, inner)
        .eq(&Poll::Ready(().into())); // todo, we here to ignore the None case
    }

    r |= inner.poll_next_unpin(cx).eq(&Poll::Ready(().into()));
    if r {
      Poll::Ready(().into())
    } else {
      Poll::Pending
    }
  }
}

impl<T: Stream + Sized> ReactiveUpdateNesterStreamExt for T {}

pub trait ReactiveUpdateNesterStreamExt: Stream + Sized {
  fn bind<F>(self, updater: F) -> StreamToReactiveUpdater<F, Self> {
    StreamToReactiveUpdater {
      updater,
      stream: self,
    }
  }
}

pub struct StreamToReactiveUpdater<F, S> {
  updater: F,
  stream: S,
}

impl<C, F, S, T> ReactiveUpdateNester<C> for StreamToReactiveUpdater<F, S>
where
  S: Stream<Item = T> + Unpin,
  C: Stream<Item = ()> + Unpin,
  F: Fn(&mut C, T),
  Self: Unpin,
{
  fn poll_update_inner(
    mut self: Pin<&mut Self>,
    cx: &mut Context<'_>,
    inner: &mut C,
  ) -> Poll<Option<()>> {
    let mut r = self
      .stream
      .poll_next_unpin(cx)
      .map(|v| {
        v.map(|v| {
          (self.updater)(inner, v);
        })
      })
      .eq(&Poll::Ready(().into())); // todo, we here to ignore the None case

    r |= inner.poll_next_unpin(cx).eq(&Poll::Ready(().into()));
    if r {
      Poll::Ready(().into())
    } else {
      Poll::Pending
    }
  }
}
