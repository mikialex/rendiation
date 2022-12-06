// https://www.youtube.com/watch?v=ePgWU3KZvfQ

use std::sync::{Arc, RwLock, RwLockReadGuard};

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
  listeners: Vec<Box<dyn Fn(&T) -> bool + Send + Sync>>,
}

impl<T> Source<T> {
  /// return should remove after triggered
  pub fn on(&mut self, cb: impl Fn(&T) -> bool + Send + Sync + 'static) -> &Self {
    self.listeners.push(Box::new(cb));
    self
  }

  #[allow(unused_must_use)]
  pub fn emit(&mut self, event: &T) {
    let mut len = self.listeners.len();
    let mut current = 0;
    // avoid any possible reallocation.
    while current < len {
      if (self.listeners[current])(event) {
        self.listeners.swap_remove(current);
        len -= 1;
      };
      current += 1;
    }
  }
}

impl<T> Default for Source<T> {
  fn default() -> Self {
    Self {
      listeners: Default::default(),
    }
  }
}

pub struct EventDispatcher<T> {
  inner: Arc<RwLock<Source<T>>>,
}

impl<T> Default for EventDispatcher<T> {
  fn default() -> Self {
    Self {
      inner: Default::default(),
    }
  }
}
impl<T> Clone for EventDispatcher<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

pub struct WeakEventDispatcher<T> {
  inner: std::sync::Weak<RwLock<Source<T>>>,
}

impl<T> WeakEventDispatcher<T> {
  pub fn emit(&self, event: &T) -> bool {
    if let Some(e) = self.inner.upgrade() {
      e.write().unwrap().emit(event);
      true
    } else {
      false
    }
  }
}

/// A stream of events.
pub struct Stream<T> {
  inner: Arc<RwLock<Source<T>>>,
}

impl<T> Clone for Stream<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<T> EventDispatcher<T> {
  pub fn emit(&self, event: &T) {
    let mut inner = self.inner.write().unwrap();
    inner.emit(event);
  }

  /// just rename, disable the ability to dispatch event
  pub fn stream(&self) -> Stream<T> {
    Stream {
      inner: self.inner.clone(),
    }
  }

  pub fn make_weak(&self) -> WeakEventDispatcher<T> {
    WeakEventDispatcher {
      inner: Arc::downgrade(&self.inner),
    }
  }
}

impl<T: 'static> Stream<T> {
  pub fn on(&self, f: impl Fn(&T) -> bool + Send + Sync + 'static) {
    self.inner.write().unwrap().on(f);
  }
  /// map a stream to another stream
  ///
  /// when the source dropped, the mapped stream will not receive any events later
  /// when self dropped, the cb in source will be remove automatically
  pub fn map<U: 'static>(&mut self, cb: impl Fn(&T) -> U + Send + Sync + 'static) -> Stream<U> {
    // dispatch default to do no allocation when created
    // as long as no one add listener, no allocation happens
    let dispatcher = EventDispatcher::<U>::default();
    let dis = dispatcher.make_weak();
    self.inner.write().unwrap().on(move |t| !dis.emit(&cb(t)));
    dispatcher.stream()
  }
  // filter
  // filter_map

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
    let dispatcher = EventDispatcher::<U>::default();
    let stream = dispatcher.stream();
    let dispatcher = dispatcher.make_weak();
    let current = Arc::new(RwLock::new(initial));
    let c = current.clone();
    self.on(move |value| {
      let mut current = c.write().unwrap();
      let changed = folder(value, &mut current);
      if changed {
        return !dispatcher.emit(&current);
      }
      false
    });
    StreamSignal { stream, current }
  }

  //todo merge
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
