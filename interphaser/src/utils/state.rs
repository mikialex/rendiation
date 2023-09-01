use std::{
  ops::Deref,
  sync::{Arc, RwLock},
};

use futures::{Stream, StreamExt};
use reactive::EventSource;

pub struct StateCell<T> {
  state: Arc<RwLock<T>>,
  events: EventSource<T>,
}

impl<T> Deref for StateCell<T> {
  type Target = EventSource<T>;
  fn deref(&self) -> &Self::Target {
    &self.events
  }
}

pub trait StateCreator: Default + 'static {
  fn use_state() -> StateCell<Self> {
    StateCell::new(Default::default())
  }
}
impl<T: Default + 'static> StateCreator for T {}

impl<T: 'static> StateCell<T> {
  pub fn new(state: T) -> Self {
    Self {
      state: Arc::new(RwLock::new(state)),
      events: Default::default(),
    }
  }

  pub fn single_listen(&self) -> impl futures::Stream<Item = T>
  where
    T: Clone + Send + Sync + 'static,
  {
    let init = self.state.read().unwrap().clone();
    self.single_listen_by(|v| v.clone(), |f| f(init))
  }

  pub fn modify_by_stream(&self, s: impl Stream<Item = T>) -> impl Stream<Item = T>
  where
    T: Clone + PartialEq,
  {
    self.modify_by_stream_by(s, |new, old| *old = new.clone())
  }

  pub fn modify_by_stream_by<X>(
    &self,
    s: impl Stream<Item = X>,
    modify: impl Fn(&X, &mut T),
  ) -> impl Stream<Item = X>
  where
    T: Clone + PartialEq,
  {
    let state = Arc::downgrade(&self.state);
    let events = self.events.clone();
    s.map(move |v| {
      if let Some(state) = state.upgrade() {
        let mut state = state.write().unwrap();
        let mut new_state = state.clone();
        modify(&v, &mut new_state);
        if new_state != *state {
          *state = new_state;
          events.emit(&state);
        }
      }
      v
    })
  }

  pub fn on_event<X>(
    &self,
    mut logic: impl FnMut(&T, &X) -> T + Send + Sync,
  ) -> impl FnMut(&X) -> bool + Send + Sync
  where
    T: Send + Sync + 'static,
  {
    let state = Arc::downgrade(&self.state);
    let events = self.events.clone();
    move |x| {
      if let Some(state) = state.upgrade() {
        let mut state = state.write().unwrap();
        *state = logic(&state, x);
        events.emit(&state);
        false
      } else {
        true
      }
    }
  }
}

impl<T> Clone for StateCell<T> {
  fn clone(&self) -> Self {
    Self {
      state: self.state.clone(),
      events: self.events.clone(),
    }
  }
}
