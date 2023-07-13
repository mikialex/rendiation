use std::{
  ops::Deref,
  sync::{Arc, RwLock},
};

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

pub trait StateCreator: Default {
  fn use_state() -> StateCell<Self> {
    StateCell::new(Default::default())
  }
}
impl<T: Default> StateCreator for T {}

impl<T> StateCell<T> {
  pub fn new(state: T) -> Self {
    Self {
      state: Arc::new(RwLock::new(state)),
      events: Default::default(),
    }
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
