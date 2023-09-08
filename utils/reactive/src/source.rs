use std::{marker::PhantomData, sync::Arc};

use futures::{Future, Stream};

use crate::*;

pub struct Source<T> {
  // return if should remove
  listeners: Vec<(u32, Box<dyn FnMut(&T) -> bool + Send + Sync>)>,
  next_id: u32, // u32 should be enough
}

pub struct RemoveToken<T> {
  handle: u32,
  phantom: PhantomData<T>,
}

impl<T> Clone for RemoveToken<T> {
  fn clone(&self) -> Self {
    Self {
      handle: self.handle,
      phantom: PhantomData,
    }
  }
}
impl<T> Copy for RemoveToken<T> {}

impl<T> Source<T> {
  /// return should be removed from source after emitted
  pub fn on(&mut self, cb: impl FnMut(&T) -> bool + Send + Sync + 'static) -> RemoveToken<T> {
    self.next_id += 1;
    self.listeners.push((self.next_id, Box::new(cb)));
    RemoveToken {
      handle: self.next_id,
      phantom: PhantomData,
    }
  }
  pub fn off(&mut self, token: RemoveToken<T>) {
    let idx = self
      .listeners
      .iter()
      .position(|v| v.0 == token.handle)
      .expect("event source remove failed");
    let _ = self.listeners.swap_remove(idx);
  }

  #[allow(unused_must_use)]
  pub fn emit(&mut self, event: &T) {
    let mut idx = 0;
    while idx < self.listeners.len() {
      let listener = &mut self.listeners[idx].1;
      if listener(event) {
        self.listeners.swap_remove(idx);
      } else {
        idx += 1;
      }
    }
  }
}

impl<T> Default for Source<T> {
  fn default() -> Self {
    Self {
      listeners: Default::default(),
      next_id: 0,
    }
  }
}

/// a simple event dispatcher.
pub struct EventSource<T> {
  /// the source is alway need mutable access, so we not use rwlock here
  inner: Arc<Mutex<Source<T>>>,
}

impl<T> Default for EventSource<T> {
  // default not to do any allocation when created
  // as long as no one add listener, no allocation happens
  fn default() -> Self {
    Self {
      inner: Default::default(),
    }
  }
}

impl<T> Clone for EventSource<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<T: 'static> EventSource<T> {
  pub fn make_weak(&self) -> WeakSource<T> {
    WeakSource {
      inner: Arc::downgrade(&self.inner),
    }
  }

  pub fn emit(&self, event: &T) {
    let mut inner = self.inner.lock().unwrap();
    inner.emit(event);
  }

  /// return should be removed from source after emitted
  pub fn on(&self, f: impl FnMut(&T) -> bool + Send + Sync + 'static) -> RemoveToken<T> {
    self.inner.lock().unwrap().on(f)
  }

  pub fn off(&self, token: RemoveToken<T>) {
    self.inner.lock().unwrap().off(token)
  }

  pub fn any_triggered(&self) -> impl futures::Stream<Item = ()> {
    self.single_listen_by(|_| (), |_| ())
  }

  pub fn single_listen(&self) -> impl futures::Stream<Item = T>
  where
    T: Clone + Send + Sync,
  {
    self.single_listen_by(|v| v.clone(), |_| {})
  }

  pub fn unbound_listen(&self) -> impl futures::Stream<Item = T>
  where
    T: Clone + Send + Sync,
  {
    self.unbound_listen_by(|v| v.clone(), |_| {})
  }

  pub fn batch_listen(&self) -> impl futures::Stream<Item = Vec<T>>
  where
    T: Clone + Send + Sync,
  {
    self.batch_listen_by(|v| v.clone(), |_| {})
  }

  pub fn unbound_listen_by<U>(
    &self,
    mapper: impl Fn(&T) -> U + Send + Sync + 'static,
    init: impl FnOnce(&dyn Fn(U)),
  ) -> impl futures::Stream<Item = U>
  where
    U: Send + Sync + 'static,
  {
    self.listen_by::<DefaultUnboundChannel, _, U>(mapper, init)
  }

  pub fn batch_listen_by<U>(
    &self,
    mapper: impl Fn(&T) -> U + Send + Sync + 'static,
    init: impl FnOnce(&dyn Fn(U)),
  ) -> impl futures::Stream<Item = Vec<U>>
  where
    U: Send + Sync + 'static,
  {
    self.listen_by::<DefaultBatchChannel, _, Vec<U>>(mapper, init)
  }

  pub fn single_listen_by<U>(
    &self,
    mapper: impl Fn(&T) -> U + Send + Sync + 'static,
    init: impl FnOnce(&dyn Fn(U)),
  ) -> impl futures::Stream<Item = U> + 'static
  where
    U: Send + Sync + 'static,
  {
    self.listen_by::<DefaultSingleValueChannel, _, U>(mapper, init)
  }

  pub fn listen_by<C, U, N>(
    &self,
    mapper: impl Fn(&T) -> U + Send + Sync + 'static,
    init: impl FnOnce(&dyn Fn(U)),
  ) -> impl futures::Stream<Item = N> + 'static
  where
    U: Send + Sync + 'static,
    C: ChannelLike<U, Message = N>,
  {
    let (sender, receiver) = C::build();
    let init_sends = |to_send| {
      C::send(&sender, to_send);
    };
    init(&init_sends);
    let remove_token = self.on(move |v| {
      C::send(&sender, mapper(v));
      C::is_closed(&sender)
    });
    let dropper = EventSourceDropper::new(remove_token, self.make_weak());
    EventSourceStream::new(dropper, receiver)
  }

  pub fn once_future<R>(
    &mut self,
    f: impl FnOnce(&T) -> R + Send + Sync + 'static,
  ) -> impl Future<Output = R>
  where
    T: Send + Sync,
    R: Send + Sync + 'static,
  {
    use futures::FutureExt;
    let f = Mutex::new(Some(f));
    let f = move |p: &_| f.lock().unwrap().take().map(|f| f(p));
    let any = self.single_listen_by(f, |_| {});
    any.into_future().map(|(r, _)| r.unwrap().unwrap())
  }
}

#[pin_project::pin_project]
pub struct EventSourceStream<T, S> {
  dropper: EventSourceDropper<T>,
  #[pin]
  stream: S,
}

impl<T, S> EventSourceStream<T, S> {
  pub fn new(dropper: EventSourceDropper<T>, stream: S) -> Self {
    Self { dropper, stream }
  }
}

impl<T, S> Stream for EventSourceStream<T, S>
where
  S: Stream,
{
  type Item = S::Item;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    self.project().stream.poll_next(cx)
  }
}

pub struct EventSourceDropper<T> {
  remove_token: RemoveToken<T>,
  weak: WeakSource<T>,
}

impl<T> EventSourceDropper<T> {
  pub fn new(remove_token: RemoveToken<T>, weak: WeakSource<T>) -> Self {
    Self { remove_token, weak }
  }
}

impl<T> Drop for EventSourceDropper<T> {
  fn drop(&mut self) {
    if let Some(source) = self.weak.inner.upgrade() {
      // it's safe to remove again here (has no effect)
      source.lock().unwrap().off(self.remove_token)
    }
  }
}

pub struct WeakSource<T> {
  inner: std::sync::Weak<Mutex<Source<T>>>,
}

impl<T> Clone for WeakSource<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<T> WeakSource<T> {
  pub fn emit(&self, event: &T) -> bool {
    if let Some(e) = self.inner.upgrade() {
      e.lock().unwrap().emit(event);
      true
    } else {
      false
    }
  }
  pub fn is_exist(&self) -> bool {
    self.inner.upgrade().is_some()
  }
}

pub struct OnceSource<T> {
  listeners: Vec<Box<dyn FnOnce(&T) + Send + Sync>>,
}

impl<T> OnceSource<T> {
  pub fn emit(&mut self, item: &T) {
    self.listeners.drain(..).for_each(|cb| cb(item))
  }

  pub fn on(&mut self, cb: impl FnOnce(&T) + Send + Sync + 'static) {
    self.listeners.push(Box::new(cb))
  }
}

impl<T> Default for OnceSource<T> {
  fn default() -> Self {
    Self {
      listeners: Default::default(),
    }
  }
}

pub struct EventOnceSource<T> {
  /// the source is alway need mutable access, so we not use rwlock here
  inner: Arc<Mutex<OnceSource<T>>>,
}

impl<T> Default for EventOnceSource<T> {
  fn default() -> Self {
    Self {
      inner: Default::default(),
    }
  }
}

impl<T> EventOnceSource<T> {
  pub fn emit(&self, item: &T) {
    let mut inner = self.inner.lock().unwrap();
    inner.emit(item);
  }

  pub fn on(&self, f: impl FnOnce(&T) + Send + Sync + 'static) {
    self.inner.lock().unwrap().on(f)
  }
}
