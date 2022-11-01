use std::sync::{Arc, RwLock};

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
  listeners: Vec<Box<dyn Fn(&T)>>,
}

impl<T> Source<T> {
  pub fn on(&mut self, cb: impl Fn(&T) + 'static) -> &Self {
    self.listeners.push(Box::new(cb));
    self
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

pub struct Stream<T> {
  inner: Arc<RwLock<Source<T>>>,
}

impl<T> EventDispatcher<T> {
  pub fn emit(&self, event: T) {
    let inner = self.inner.write().unwrap();
    for listener in &inner.listeners {
      listener(&event)
    }
  }

  pub fn stream(&self) -> Stream<T> {
    Stream {
      inner: self.inner.clone(),
    }
  }
}

impl<T> Stream<T> {
  pub fn map<U: 'static>(&mut self, cb: impl Fn(&T) -> U + 'static) -> Stream<U> {
    let dispatcher = EventDispatcher::<U>::default();
    let dis = dispatcher.clone();
    self.inner.write().unwrap().on(move |t| dis.emit(cb(t)));
    dispatcher.stream()
  }
  // filter
  // filter_map
}
