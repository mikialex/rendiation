// https://www.youtube.com/watch?v=ePgWU3KZvfQ

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
  // return if should remove
  listeners: Vec<Box<dyn Fn(&T) -> bool>>,
}

impl<T> Source<T> {
  pub fn on(&mut self, cb: impl Fn(&T) -> bool + 'static) -> &Self {
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

/// A stream of events.
pub struct Stream<T> {
  inner: Arc<RwLock<Source<T>>>,
}

impl<T> EventDispatcher<T> {
  #[allow(unused_must_use)]
  pub fn emit(&self, event: &T) {
    let mut inner = self.inner.write().unwrap();
    let mut len = inner.listeners.len();
    let mut current = 0;
    // avoid any possible reallocation.
    while current < len {
      if (inner.listeners[current])(event) {
        inner.listeners.swap_remove(current);
        len -= 1;
      };
      current += 1;
    }
  }

  /// just rename, without the ability to dispatch event
  pub fn stream(&self) -> Stream<T> {
    Stream {
      inner: self.inner.clone(),
    }
  }
}

impl<T> Stream<T> {
  /// map a stream to another stream
  ///
  /// when the source dropped, the mapped stream will not receive any events later
  pub fn map<U: 'static>(&mut self, cb: impl Fn(&T) -> U + 'static) -> Stream<U> {
    // dispatch default to do no allocation when created
    let dispatcher = EventDispatcher::<U>::default();
    let dis = dispatcher.clone(); // todo weak
    self.inner.write().unwrap().on(move |t| {
      dis.emit(&cb(t));
      false
    });
    dispatcher.stream()
  }
  // filter
  // filter_map

  // pub fn hold(&self, initial: T) -> impl Signal<Output = T> {
  //   todo!()
  // }

  // pub fn fold(&self, initial: T) -> impl Signal<Output = T> {
  //   todo!()
  // }

  pub fn merge(&self, other: &Self) -> Self {
    todo!()
  }
}
