use crate::*;

pub struct StreamBroadcaster<S, D, F> {
  inner: Arc<RwLock<StreamBroadcasterImpl<S, D, F>>>,
}

impl<S, D, F, I> Stream for StreamBroadcaster<S, D, F>
where
  S: Stream<Item = I> + Unpin,
  S::Item: Clone,
  F: BroadcastBehavior<I, D>,
{
  type Item = I;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let mut inner = self.inner.write();
    let inner: &mut StreamBroadcasterImpl<_, _, _> = &mut inner;
    let inner = Pin::new(inner);
    inner.poll_next(cx)
  }
}

impl<S, D, F> Clone for StreamBroadcaster<S, D, F> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<S, D, F> StreamBroadcaster<S, D, F> {
  pub fn new(source: S, broad_cast: F) -> Self {
    let inner = StreamBroadcasterImpl {
      source,
      distributer: Default::default(),
      broad_cast,
    };
    let inner = Arc::new(RwLock::new(inner));
    Self { inner }
  }
}

#[pin_project]
struct StreamBroadcasterImpl<S, D, F> {
  #[pin]
  source: S,
  distributer: Vec<Option<futures::channel::mpsc::UnboundedSender<D>>>,
  broad_cast: F,
}

impl<S, D, F, I> Stream for StreamBroadcasterImpl<S, D, F>
where
  S: Stream<Item = I> + Unpin,
  S::Item: Clone,
  F: BroadcastBehavior<I, D>,
{
  type Item = I;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();
    if let Poll::Ready(v) = this.source.poll_next(cx) {
      if let Some(input) = v {
        F::broad_cast(input.clone(), this.distributer);
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
pub struct BroadcastedStream<S, D, F> {
  #[pin]
  rev: futures::channel::mpsc::UnboundedReceiver<D>,
  index: usize,
  source: Arc<RwLock<StreamBroadcasterImpl<S, D, F>>>,
}

pub trait BroadcastBehavior<I, O> {
  fn broad_cast(input: I, output: &mut Vec<Option<futures::channel::mpsc::UnboundedSender<O>>>);
}

impl<S, D, F> Stream for BroadcastedStream<S, D, F>
where
  S: Stream + Unpin,
  F: BroadcastBehavior<S::Item, D>,
{
  type Item = D;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let outer_this = self.project();
    let mut inner = outer_this.source.write();
    let inner: &mut StreamBroadcasterImpl<_, _, _> = &mut inner;
    let inner = Pin::new(inner);
    let mut this = inner.project();
    // must use while let here, because we rely on this to update all depend system
    while let Poll::Ready(v) = this.source.as_mut().poll_next(cx) {
      if let Some(input) = v {
        F::broad_cast(input, this.distributer);
      } else {
        // forward early termination
        return Poll::Ready(None);
      }
    }

    outer_this.rev.poll_next(cx)
  }
}

impl<S, D> StreamBroadcaster<S, D, FanOut>
where
  S: Stream<Item = D> + Unpin,
{
  pub fn fork_stream_with_init(
    &self,
    init: impl IntoIterator<Item = D>,
  ) -> BroadcastedStream<S, D, FanOut> {
    let mut inner = self.inner.write();
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
    init.into_iter().for_each(|init_delta| {
      sender.unbounded_send(init_delta).ok();
    });
    inner.distributer[index] = sender.into();
    BroadcastedStream {
      rev,
      index,
      source: self.inner.clone(),
    }
  }
  pub fn fork_stream(&self) -> BroadcastedStream<S, D, FanOut> {
    self.fork_stream_with_init([])
  }
}

impl<S, D> StreamBroadcaster<S, D, IndexMapping>
where
  S: Stream<Item = (usize, D)> + Unpin,
{
  pub fn create_sub_stream_by_index(&self, index: usize) -> BroadcastedStream<S, D, IndexMapping> {
    let mut inner = self.inner.write();
    // todo shrink logic?
    while inner.distributer.len() <= index {
      inner.distributer.push(None);
    }
    let (sender, rev) = futures::channel::mpsc::unbounded();
    inner.distributer[index] = sender.into();
    BroadcastedStream {
      rev,
      index,
      source: self.inner.clone(),
    }
  }
}

pub struct IndexMapping;
impl<O> BroadcastBehavior<(usize, O), O> for IndexMapping {
  fn broad_cast(
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
impl<I: Clone> BroadcastBehavior<I, I> for FanOut {
  fn broad_cast(input: I, output: &mut Vec<Option<futures::channel::mpsc::UnboundedSender<I>>>) {
    output.iter_mut().for_each(|sender| {
      if let Some(sender_real) = sender {
        if sender_real.unbounded_send(input.clone()).is_err() {
          *sender = None;
        }
      }
    })
  }
}

#[test]
fn test_fan_out() {
  let (send, rev) = futures::channel::mpsc::unbounded::<u32>();
  send.unbounded_send(10).unwrap();
  send.unbounded_send(3).unwrap();

  let caster = rev.create_broad_caster();
  let mut a = caster.fork_stream();
  let mut b = caster.fork_stream();

  let mut a_c = 0;
  let mut b_c = 0;
  do_updates(&mut a, |_| a_c += 1);
  assert_eq!(a_c, 2);
  do_updates(&mut b, |_| b_c += 1);
  assert_eq!(b_c, 2);

  send.unbounded_send(3).unwrap();

  do_updates(&mut a, |_| a_c += 1);
  assert_eq!(a_c, 3);
  do_updates(&mut b, |_| b_c += 1);
  assert_eq!(b_c, 3);

  let mut c = caster.fork_stream();
  let mut c_c = 0;

  do_updates(&mut c, |_| c_c += 1);
  assert_eq!(c_c, 0);
  send.unbounded_send(3).unwrap();

  do_updates(&mut c, |_| c_c += 1);
  assert_eq!(c_c, 1);
}

#[test]
fn test_indexed() {
  let (send, rev) = futures::channel::mpsc::unbounded::<(usize, u32)>();
  send.unbounded_send((0, 10)).unwrap();
  send.unbounded_send((0, 3)).unwrap();

  let mut caster = rev.create_index_mapping_broadcaster();

  let mut a = caster.create_sub_stream_by_index(0);
  let mut b = caster.create_sub_stream_by_index(1);

  let mut a_c = 0;
  let mut b_c = 0;
  do_updates(&mut caster, |_| {});
  do_updates(&mut a, |_| a_c += 1);
  assert_eq!(a_c, 2);
  do_updates(&mut caster, |_| {});
  do_updates(&mut b, |_| b_c += 1);
  assert_eq!(b_c, 0);

  send.unbounded_send((1, 3)).unwrap();

  do_updates(&mut caster, |_| {});
  do_updates(&mut a, |_| a_c += 1);
  assert_eq!(a_c, 2);
  do_updates(&mut caster, |_| {});
  do_updates(&mut b, |_| b_c += 1);
  assert_eq!(b_c, 1);

  let mut c = caster.create_sub_stream_by_index(0);
  let mut c_c = 0;

  do_updates(&mut caster, |_| {});
  do_updates(&mut c, |_| c_c += 1);
  assert_eq!(c_c, 0);
  send.unbounded_send((0, 3)).unwrap();

  do_updates(&mut caster, |_| {});
  do_updates(&mut c, |_| c_c += 1);
  assert_eq!(c_c, 1);
}
