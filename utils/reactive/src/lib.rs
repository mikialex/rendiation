use std::sync::{Arc, RwLock, RwLockReadGuard};

use arena::{Arena, Handle};

/// container for values that change (discretely) over time.
pub trait Signal {
  type Output;
  fn sample(&self) -> Self::Output;
}

struct SignalMap<T, F> {
  inner: T,
  mapper: F,
}

impl<S, U, T, F> Signal for SignalMap<T, F>
where
  T: Signal<Output = S>,
  F: Fn(S) -> U,
{
  type Output = U;

  fn sample(&self) -> Self::Output {
    (self.mapper)(self.inner.sample())
  }
}

pub struct Source<T> {
  // return if should remove
  listeners: Arena<Box<dyn Fn(&T) -> bool + Send + Sync>>,
}

pub struct RemoveToken<T> {
  handle: Handle<Box<dyn Fn(&T) -> bool + Send + Sync>>,
}

impl<T> Clone for RemoveToken<T> {
  fn clone(&self) -> Self {
    Self {
      handle: self.handle.clone(),
    }
  }
}
impl<T> Copy for RemoveToken<T> {}

impl<T> Source<T> {
  /// return should remove after triggered
  pub fn on(&mut self, cb: impl Fn(&T) -> bool + Send + Sync + 'static) -> RemoveToken<T> {
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
pub struct Stream<T> {
  inner: Arc<RwLock<Source<T>>>,
}

impl<T> Default for Stream<T> {
  fn default() -> Self {
    Self {
      inner: Default::default(),
    }
  }
}

impl<T> Clone for Stream<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

pub struct WeakStream<T> {
  inner: std::sync::Weak<RwLock<Source<T>>>,
}

impl<T> Clone for WeakStream<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<T> WeakStream<T> {
  pub fn emit(&self, event: &T) -> bool {
    if let Some(e) = self.inner.upgrade() {
      e.write().unwrap().emit(event);
      true
    } else {
      false
    }
  }
}

impl<T: 'static> Stream<T> {
  pub fn make_weak(&self) -> WeakStream<T> {
    WeakStream {
      inner: Arc::downgrade(&self.inner),
    }
  }

  pub fn emit(&self, event: &T) {
    let mut inner = self.inner.write().unwrap();
    inner.emit(event);
  }

  /// return should remove after triggered
  pub fn on(&self, f: impl Fn(&T) -> bool + Send + Sync + 'static) -> RemoveToken<T> {
    self.inner.write().unwrap().on(f)
  }

  pub fn off(&mut self, token: RemoveToken<T>) {
    self.inner.write().unwrap().off(token)
  }

  /// map a stream to another stream
  ///
  /// when the source dropped, the mapped stream will not receive any events later
  ///
  /// when self dropped, the cb in source will be removed automatically
  pub fn map<U: 'static>(&self, cb: impl Fn(&T) -> U + Send + Sync + 'static) -> Stream<U> {
    // default to do no allocation when created
    // as long as no one add listener, no allocation happens
    let stream = Stream::<U>::default();
    let weak = stream.make_weak();
    self.inner.write().unwrap().on(move |t| !weak.emit(&cb(t)));
    stream
  }

  pub fn filter(&self, _cb: impl Fn(&T) -> bool + Send + Sync + 'static) -> Stream<T> {
    todo!()
  }

  pub fn filter_map<U: 'static>(
    &self,
    _cb: impl Fn(&T) -> Option<U> + Send + Sync + 'static,
  ) -> Stream<U> {
    todo!()
  }

  pub fn hold(&self, initial: T) -> StreamSignal<T>
  where
    T: Clone + Send + Sync,
  {
    let stream = self.clone();
    let current = Arc::new(RwLock::new(initial));
    let c = Arc::downgrade(&current);
    stream.on(move |value| {
      if let Some(c) = c.upgrade() {
        *c.write().unwrap() = value.clone();
        false
      } else {
        true
      }
    });
    StreamSignal { stream, current }
  }

  pub fn fold<U, F>(&self, initial: U, folder: F) -> StreamSignal<U>
  where
    F: Fn(&T, &mut U) -> bool + Send + Sync + 'static, // return if changed
    U: 'static + Send + Sync,
  {
    let stream = Stream::<U>::default();
    let weak = stream.make_weak();
    let current = Arc::new(RwLock::new(initial));
    let c = current.clone();
    self.on(move |value| {
      let mut current = c.write().unwrap();
      let changed = folder(value, &mut current);
      if changed {
        return !weak.emit(&current);
      }
      false
    });
    StreamSignal { stream, current }
  }
}

impl<T: 'static> Stream<Stream<T>> {
  pub fn flatten(&self) -> Stream<T> {
    let stream = Stream::<T>::default();

    let weak = stream.make_weak();
    let previous_stream: Arc<RwLock<Option<(Stream<T>, RemoveToken<T>)>>> = Default::default();

    self.on(move |new_stream| {
      let mut previous_stream = previous_stream.write().unwrap();
      let previous_stream: &mut Option<(Stream<T>, RemoveToken<T>)> = &mut previous_stream;
      if let Some((previous_stream, token)) = previous_stream {
        previous_stream.off(*token);
      }
      let weak = weak.clone();
      let token = new_stream.on(move |value| !weak.emit(value));
      *previous_stream = Some((new_stream.clone(), token));

      false
    });
    stream
  }
}

pub struct StreamSignal<T> {
  stream: Stream<T>,
  current: Arc<RwLock<T>>,
}

impl<T: Clone> Signal for StreamSignal<T> {
  type Output = T;
  fn sample(&self) -> Self::Output {
    self.current.read().unwrap().clone()
  }
}

impl<T> StreamSignal<T> {
  pub fn as_stream(&self) -> &Stream<T> {
    &self.stream
  }

  pub fn get_guard(&self) -> RwLockReadGuard<T> {
    self.current.read().unwrap()
  }
}
