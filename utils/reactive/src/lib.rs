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
  /// return should remove after triggered
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

impl<T> Clone for Stream<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
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

  /// just rename, disable the ability to dispatch event
  pub fn stream(&self) -> Stream<T> {
    Stream {
      inner: self.inner.clone(),
    }
  }
}

impl<T: 'static> Stream<T> {
  pub fn on(&self, f: impl Fn(&T) -> bool + 'static) {
    self.inner.write().unwrap().on(f);
  }
  /// map a stream to another stream
  ///
  /// when the source dropped, the mapped stream will not receive any events later
  pub fn map<U: 'static>(&mut self, cb: impl Fn(&T) -> U + 'static) -> Stream<U> {
    // dispatch default to do no allocation when created
    // as long as no one add listener, no allocation happens
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

  pub fn hold(&self, initial: T) -> StreamSignal<T>
  where
    T: Clone,
  {
    let stream = self.clone();
    let current = Arc::new(RwLock::new(initial));
    let c = current.clone();
    stream.on(move |value| {
      *c.write().unwrap() = value.clone();
      false
    });
    StreamSignal { stream, current }
  }

  pub fn fold<U, F>(&self, initial: U, folder: F) -> StreamSignal<U>
  where
    F: Fn(&T, &mut U) -> bool + 'static, // return if changed
    U: 'static,
  {
    let dispatcher = EventDispatcher::<U>::default();
    let d = dispatcher.clone();
    let current = Arc::new(RwLock::new(initial));
    let c = current.clone();
    self.on(move |value| {
      let mut current = c.write().unwrap();
      let changed = folder(value, &mut current);
      if changed {
        dispatcher.emit(&current);
      }
      false
    });
    StreamSignal {
      stream: d.stream(),
      current,
    }
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
}
