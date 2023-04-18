use arena::{Arena, Handle};
use futures::Future;
use std::sync::{Arc, RwLock};

pub struct Source<T> {
  // return if should remove
  listeners: Arena<Box<dyn FnMut(&T) -> bool + Send + Sync>>,
}

impl<T: Clone + Send + Sync + 'static> EventSource<T> {
  pub fn listen(&self) -> impl futures::Stream<Item = T> {
    let (sender, receiver) = futures::channel::mpsc::unbounded();
    self.on(move |v| sender.unbounded_send(v.clone()).is_err());
    receiver
  }
}

pub struct RemoveToken<T> {
  handle: Handle<Box<dyn FnMut(&T) -> bool + Send + Sync>>,
}

impl<T> Clone for RemoveToken<T> {
  fn clone(&self) -> Self {
    Self {
      handle: self.handle,
    }
  }
}
impl<T> Copy for RemoveToken<T> {}

impl<T> Source<T> {
  /// return should remove after triggered
  pub fn on(&mut self, cb: impl FnMut(&T) -> bool + Send + Sync + 'static) -> RemoveToken<T> {
    let handle = self.listeners.insert(Box::new(cb));
    RemoveToken { handle }
  }
  pub fn off(&mut self, token: RemoveToken<T>) {
    self.listeners.remove(token.handle);
  }

  #[allow(unused_must_use)]
  pub fn emit(&mut self, event: &T) {
    // todo avoid any possible allocation.
    let mut to_remove = Vec::with_capacity(0);
    self.listeners.iter_mut().for_each(|(handle, cb)| {
      if cb(event) {
        to_remove.push(handle)
      }
    });
    to_remove.drain(..).for_each(|handle| {
      self.listeners.remove(handle);
    })
  }
}

impl<T> Default for Source<T> {
  fn default() -> Self {
    Self {
      listeners: Default::default(),
    }
  }
}

/// A stream of events.
pub struct EventSource<T> {
  inner: Arc<RwLock<Source<T>>>,
}

impl<T> Default for EventSource<T> {
  // default to do no allocation when created
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
    let mut inner = self.inner.write().unwrap();
    inner.emit(event);
  }

  /// return should remove after triggered
  pub fn on(&self, f: impl FnMut(&T) -> bool + Send + Sync + 'static) -> RemoveToken<T> {
    self.inner.write().unwrap().on(f)
  }

  pub fn off(&mut self, token: RemoveToken<T>) {
    self.inner.write().unwrap().off(token)
  }

  pub fn listen_by<U: Send + Sync + 'static>(
    &self,
    mapper: impl Fn(&T) -> U + Send + Sync + 'static,
    init: U,
  ) -> impl futures::Stream<Item = U> {
    let (sender, receiver) = futures::channel::mpsc::unbounded();
    sender.unbounded_send(init).ok();
    self.on(move |v| {
      sender.unbounded_send(mapper(v)).ok();
      sender.is_closed()
    });
    receiver
  }

  pub fn once_future(&mut self) -> impl Future<Output = Option<T>>
  where
    T: Clone + Send + Sync,
  {
    use futures::FutureExt;
    let (s, r) = futures::channel::oneshot::channel::<T>();
    let mut s = Some(s);
    self.on(move |re| {
      s.take().map(|s| s.send(re.clone()).ok());
      true
    });
    r.map(|v| v.ok())
  }
}

pub struct WeakSource<T> {
  inner: std::sync::Weak<RwLock<Source<T>>>,
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
      e.write().unwrap().emit(event);
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
  inner: Arc<RwLock<OnceSource<T>>>,
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
    let mut inner = self.inner.write().unwrap();
    inner.emit(item);
  }

  pub fn on(&self, f: impl FnOnce(&T) + Send + Sync + 'static) {
    self.inner.write().unwrap().on(f)
  }
}
