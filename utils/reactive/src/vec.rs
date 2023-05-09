use crate::*;

use futures::StreamExt;

#[pin_project]
pub struct StreamVec<T> {
  streams: Vec<Option<T>>,
  waked: Arc<RwLock<Vec<usize>>>,
  waker: Arc<RwLock<Option<Waker>>>,
}

impl<T> Default for StreamVec<T> {
  fn default() -> Self {
    Self {
      streams: Default::default(),
      waked: Default::default(),
      waker: Default::default(),
    }
  }
}

impl<T> StreamVec<T> {
  pub fn get(&self, index: usize) -> Option<&T> {
    if let Some(inner) = self.streams.get(index) {
      inner.as_ref()
    } else {
      None
    }
  }

  pub fn insert(&mut self, index: usize, st: Option<T>) {
    // assure allocated
    while self.streams.len() <= index {
      self.streams.push(None);
    }
    self.streams[index] = st;
    self.waked.write().unwrap().push(index);
    self.try_wake()
  }

  pub fn try_wake(&self) {
    let waker = self.waker.read().unwrap();
    let waker: &Option<_> = &waker;
    if let Some(waker) = waker {
      waker.wake_by_ref();
    }
  }
}

pub struct IndexedItem<T> {
  pub index: usize,
  pub item: T,
}

pub(crate) struct ChangeWaker<T> {
  pub(crate) index: T,
  pub(crate) changed: Arc<RwLock<Vec<T>>>,
  pub(crate) waker: Arc<RwLock<Option<Waker>>>,
}

impl<T: Send + Sync + Clone> futures::task::ArcWake for ChangeWaker<T> {
  fn wake_by_ref(arc_self: &Arc<Self>) {
    arc_self
      .changed
      .write()
      .unwrap()
      .push(arc_self.index.clone());
    let waker = arc_self.waker.read().unwrap();
    let waker: &Option<_> = &waker;
    if let Some(waker) = waker {
      waker.wake_by_ref();
    }
  }
}

impl<T: Stream + Unpin> Stream for StreamVec<T> {
  type Item = IndexedItem<T::Item>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let this = self.project();

    this.waker.write().unwrap().replace(cx.waker().clone());

    loop {
      let last = this.waked.read().unwrap().last().copied();
      if let Some(index) = last {
        let waker = Arc::new(ChangeWaker {
          waker: this.waker.clone(),
          index,
          changed: this.waked.clone(),
        });
        let waker = futures::task::waker_ref(&waker);
        let mut cx = Context::from_waker(&waker);

        if let Some(stream) = this.streams.get_mut(index).unwrap() {
          if let Poll::Ready(r) = stream
            .poll_next_unpin(&mut cx)
            .map(|r| r.map(|item| IndexedItem { index, item }))
          {
            if r.is_none() {
              this.streams[index] = None;
            } else {
              return Poll::Ready(r);
            }
          }
        }

        this.waked.write().unwrap().pop().unwrap();
      } else {
        break;
      }
    }

    // while let Some(index) = this.waked.read().unwrap().last().copied() {
    //   // let waker = Arc::new(ChangeWaker {
    //   //   waker: this.waker.clone(),
    //   //   index,
    //   //   changed: this.waked.clone(),
    //   // });
    //   // let waker = futures::task::waker_ref(&waker);
    //   // let mut cx = Context::from_waker(&waker);

    //   // if let Some(stream) = this.streams.get_mut(index).unwrap() {
    //   //   if let Poll::Ready(r) = stream
    //   //     .poll_next_unpin(&mut cx)
    //   //     .map(|r| r.map(|item| IndexedItem { index, item }))
    //   //   {
    //   //     if r.is_none() {
    //   //       this.streams[index] = None;
    //   //     } else {
    //   //       return Poll::Ready(r);
    //   //     }
    //   //   }
    //   // }

    //   this.waked.write().unwrap().pop();
    // }
    Poll::Pending
  }
}

#[test]
fn should_drain() {
  let (s, r) = futures::channel::mpsc::unbounded::<u32>();

  s.unbounded_send(1).ok();
  s.unbounded_send(2).ok();

  let mut stream = StreamVec::default();
  stream.insert(0, Some(r));

  let mut c = 0;
  crate::do_updates(&mut stream, |_| c += 1);
  assert_eq!(c, 2);
}

#[pin_project]
pub struct MergeIntoStreamVec<S, T> {
  #[pin]
  inner: S,
  #[pin]
  vec: StreamVec<T>,
}

impl<S, T> AsRef<StreamVec<T>> for MergeIntoStreamVec<S, T> {
  fn as_ref(&self) -> &StreamVec<T> {
    &self.vec
  }
}

impl<S, T> MergeIntoStreamVec<S, T> {
  pub fn new(inner: S) -> Self {
    Self {
      inner,
      vec: Default::default(),
    }
  }
}

#[derive(Clone, Debug)]
pub enum VecUpdateUnit<T> {
  Remove(usize),
  Active(usize),
  Update { index: usize, item: T },
}

impl<S, T> Stream for MergeIntoStreamVec<S, T>
where
  S: Stream<Item = (usize, Option<T>)>,
  T: Stream + Unpin,
{
  type Item = VecUpdateUnit<T::Item>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    let mut this = self.project();

    if let Poll::Ready(next) = this.inner.poll_next(cx) {
      if let Some((index, result)) = next {
        let r = if result.is_some() {
          VecUpdateUnit::Active(index)
        } else {
          VecUpdateUnit::Remove(index)
        };
        this.vec.insert(index, result);
        return Poll::Ready(Some(r));
      } else {
        return Poll::Ready(None);
      }
    } else {
      // the vec will never terminated
      if let Poll::Ready(Some(IndexedItem { index, item })) = this.vec.poll_next(cx) {
        return Poll::Ready(Some(VecUpdateUnit::Update { index, item }));
      }
    }

    Poll::Pending
  }
}
