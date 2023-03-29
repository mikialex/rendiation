use std::{
  pin::Pin,
  sync::{Arc, RwLock},
  task::Poll,
};

use futures::Stream;
use pin_project::pin_project;

pub struct StreamVecDistributer<S, D> {
  inner: Arc<RwLock<StreamVecDistributerInner<S, D>>>,
}

#[pin_project]
struct StreamVecDistributerInner<S, D> {
  #[pin]
  source: S,
  distributer: Vec<Option<futures::channel::mpsc::UnboundedSender<D>>>,
}

#[pin_project]
struct DistributedStream<S, D> {
  #[pin]
  rev: futures::channel::mpsc::UnboundedReceiver<D>,
  source: Arc<RwLock<StreamVecDistributerInner<S, D>>>,
}

impl<S: Stream<Item = (usize, D)> + Unpin, D> Stream for DistributedStream<S, D> {
  type Item = D;

  fn poll_next(
    self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
  ) -> Poll<Option<Self::Item>> {
    let outer_this = self.project();
    let mut inner = outer_this.source.write().unwrap();
    let inner: &mut StreamVecDistributerInner<_, _> = &mut inner;
    let inner = Pin::new(inner);
    let this = inner.project();
    if let Poll::Ready(v) = this.source.poll_next(cx) {
      if let Some((index, v)) = v {
        if let Some(sender) = this.distributer.get_mut(index) {
          if let Some(sender_real) = sender {
            if sender_real.unbounded_send(v).is_err() {
              *sender = None;
            }
          }
        }
      } else {
        // forward early termination
        return Poll::Ready(None);
      }
    }

    outer_this.rev.poll_next(cx)
  }
}

impl<S, D> StreamVecDistributer<S, D> {
  pub fn create_sub_stream(&mut self, index: usize) -> impl Stream<Item = D> {
    let mut inner = self.inner.write().unwrap();
    while inner.distributer.len() > index {
      inner.distributer.push(None);
    }
    let (sender, rev) = futures::channel::mpsc::unbounded();
    inner.distributer[index] = sender.into();
    rev
  }
}
