use std::ops::Range;

use crate::*;

pub struct StreamBatchIndexer<S, T> {
  inner: Arc<RwLock<StreamBatchIndexerImpl<S, T>>>,
}

impl<S, T> StreamBatchIndexer<S, T> {
  pub fn new(source: S) -> Self {
    let inner = StreamBatchIndexerImpl {
      source,
      distributer: Default::default(),
    };
    Self {
      inner: Arc::new(RwLock::new(inner)),
    }
  }
  pub fn create_sub_stream_by_index(&self, index: usize) -> BatchIndexedStream<S, T> {
    let mut inner = self.inner.write().unwrap();
    // todo shrink logic?
    while inner.distributer.len() <= index {
      inner.distributer.push(None);
    }
    let (sender, rev) = futures::channel::mpsc::unbounded();
    inner.distributer[index] = sender.into();
    BatchIndexedStream {
      rev,
      index,
      source: self.inner.clone(),
    }
  }
}

impl<S, T> Stream for StreamBatchIndexer<S, T>
where
  S: Stream<Item = Vec<(usize, T)>> + Unpin,
  S::Item: Clone,
{
  type Item = Arc<Vec<(usize, T)>>; // return is sorted by index

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let mut inner = self.inner.write().unwrap();
    let inner: &mut StreamBatchIndexerImpl<_, _> = &mut inner;
    inner.poll_next_unpin(cx)
  }
}

#[pin_project]
struct StreamBatchIndexerImpl<S, T> {
  #[pin]
  source: S,
  distributer: Vec<Option<futures::channel::mpsc::UnboundedSender<AfterIndexedMessage<T>>>>,
}

#[derive(Clone)]
pub struct AfterIndexedMessage<T> {
  pub source: Arc<Vec<(usize, T)>>,
  pub range: Range<usize>,
}

impl<T: Clone> Iterator for AfterIndexedMessage<T> {
  type Item = T;

  fn next(&mut self) -> Option<Self::Item> {
    if self.range.is_empty() {
      None
    } else {
      self.range.start += 1;
      self.source.get(self.range.start - 1).map(|v| v.1.clone())
    }
  }
}

impl<S, T> Stream for StreamBatchIndexerImpl<S, T>
where
  S: Stream<Item = Vec<(usize, T)>> + Unpin,
{
  type Item = Arc<Vec<(usize, T)>>; // return is sorted by index

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    if let Poll::Ready(v) = this.source.poll_next(cx) {
      if let Some(mut input) = v {
        // note! here we must use stable sort to preserve message order
        input.sort_by(|a, b| a.0.cmp(&b.0));
        let input = Arc::new(input);

        let mut start = 0;
        input.group_by(|a, b| a.0 == b.0).for_each(|slice| {
          let id = slice[0].0;
          if let Some(d) = this.distributer.get_mut(id) {
            if let Some(dis) = d {
              let sub = AfterIndexedMessage {
                source: input.clone(),
                range: (start..start + slice.len()),
              };
              start += slice.len();
              if dis.unbounded_send(sub).is_err() {
                *d = None;
              }
            }
          }
        });
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
pub struct BatchIndexedStream<S, T> {
  #[pin]
  rev: futures::channel::mpsc::UnboundedReceiver<AfterIndexedMessage<T>>,
  index: usize,
  source: Arc<RwLock<StreamBatchIndexerImpl<S, T>>>,
}

impl<S, T> Stream for BatchIndexedStream<S, T>
where
  S: Stream<Item = Vec<(usize, T)>> + Unpin,
{
  type Item = AfterIndexedMessage<T>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let outer_this = self.project();
    let mut inner = outer_this.source.write().unwrap();
    let inner: &mut StreamBatchIndexerImpl<_, _> = &mut inner;
    // just trigger the upstream
    let _ = inner.poll_next_unpin(cx);

    outer_this.rev.poll_next(cx)
  }
}
