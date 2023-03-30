use std::{
  pin::Pin,
  sync::{Arc, RwLock},
  task::Poll,
};

use futures::Stream;
use pin_project::pin_project;

pub struct StreamBoardCaster<S, D, F> {
  inner: Arc<RwLock<StreamBoardCasterInner<S, D, F>>>,
}

impl<S, F> Stream for StreamBoardCaster<S, S::Item, F>
where
  S: Stream + Unpin,
  S::Item: Clone,
  F: BoardCastBehavior<S::Item, S::Item>,
{
  type Item = S::Item;

  fn poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
    let mut inner = self.inner.write().unwrap();
    let inner: &mut StreamBoardCasterInner<_, _, _> = &mut inner;
    let inner = Pin::new(inner);
    inner.poll_next(cx)
  }
}

impl<S, D, F> Clone for StreamBoardCaster<S, D, F> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<S, D, F> StreamBoardCaster<S, D, F> {
  pub fn new(source: S, board_cast: F) -> Self {
    let inner = StreamBoardCasterInner {
      source,
      distributer: Default::default(),
      board_cast,
    };
    let inner = Arc::new(RwLock::new(inner));
    Self { inner }
  }
}

#[pin_project]
struct StreamBoardCasterInner<S, D, F> {
  #[pin]
  source: S,
  distributer: Vec<Option<futures::channel::mpsc::UnboundedSender<D>>>,
  board_cast: F,
}

impl<S, F> Stream for StreamBoardCasterInner<S, S::Item, F>
where
  S: Stream + Unpin,
  S::Item: Clone,
  F: BoardCastBehavior<S::Item, S::Item>,
{
  type Item = S::Item;

  fn poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
    let this = self.project();
    if let Poll::Ready(v) = this.source.poll_next(cx) {
      if let Some(input) = v {
        F::board_cast(input.clone(), this.distributer);
        Poll::Ready(input.into())
      } else {
        // forward early termination
        Poll::Ready(None)
      }
    } else {
      Poll::Pending
    }
  }
}

#[pin_project]
pub struct BoardCastedStream<S, D, F> {
  #[pin]
  rev: futures::channel::mpsc::UnboundedReceiver<D>,
  index: usize,
  source: Arc<RwLock<StreamBoardCasterInner<S, D, F>>>,
}

pub trait BoardCastBehavior<I, O> {
  fn board_cast(input: I, output: &mut Vec<Option<futures::channel::mpsc::UnboundedSender<O>>>);
}

impl<S, D, F> Stream for BoardCastedStream<S, D, F>
where
  S: Stream + Unpin,
  F: BoardCastBehavior<S::Item, D>,
{
  type Item = D;

  fn poll_next(
    self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
  ) -> Poll<Option<Self::Item>> {
    let outer_this = self.project();
    let mut inner = outer_this.source.write().unwrap();
    let inner: &mut StreamBoardCasterInner<_, _, _> = &mut inner;
    let inner = Pin::new(inner);
    let this = inner.project();
    if let Poll::Ready(v) = this.source.poll_next(cx) {
      if let Some(input) = v {
        F::board_cast(input, this.distributer);
      } else {
        // forward early termination
        return Poll::Ready(None);
      }
    }

    outer_this.rev.poll_next(cx)
  }
}

impl<S, D> StreamBoardCaster<S, D, FanOut>
where
  S: Stream<Item = D> + Unpin,
{
  pub fn fork_stream(&self) -> BoardCastedStream<S, D, FanOut> {
    let mut inner = self.inner.write().unwrap();
    let index = inner
      .distributer
      .iter()
      .position(|v| v.is_none())
      .unwrap_or_else(|| {
        inner.distributer.push(None);
        inner.distributer.len() - 1
      });
    // todo shrink logic?
    let (sender, rev) = futures::channel::mpsc::unbounded();
    inner.distributer[index] = sender.into();
    BoardCastedStream {
      rev,
      index,
      source: self.inner.clone(),
    }
  }
}

impl<S, D> StreamBoardCaster<S, D, IndexMapping>
where
  S: Stream<Item = (usize, D)> + Unpin,
{
  pub fn create_sub_stream_by_index(&self, index: usize) -> BoardCastedStream<S, D, IndexMapping> {
    let mut inner = self.inner.write().unwrap();
    // todo shrink logic?
    while inner.distributer.len() > index {
      inner.distributer.push(None);
    }
    let (sender, rev) = futures::channel::mpsc::unbounded();
    inner.distributer[index] = sender.into();
    BoardCastedStream {
      rev,
      index,
      source: self.inner.clone(),
    }
  }
}

pub struct IndexMapping;
impl<O> BoardCastBehavior<(usize, O), O> for IndexMapping {
  fn board_cast(
    (index, v): (usize, O),
    output: &mut Vec<Option<futures::channel::mpsc::UnboundedSender<O>>>,
  ) {
    if let Some(sender) = output.get_mut(index) {
      if let Some(sender_real) = sender {
        if sender_real.unbounded_send(v).is_err() {
          *sender = None;
        }
      }
    }
  }
}

pub struct FanOut;
impl<I: Clone> BoardCastBehavior<I, I> for FanOut {
  fn board_cast(input: I, output: &mut Vec<Option<futures::channel::mpsc::UnboundedSender<I>>>) {
    output.iter_mut().for_each(|sender| {
      if let Some(sender_real) = sender {
        if sender_real.unbounded_send(input.clone()).is_err() {
          *sender = None;
        }
      }
    })
  }
}
