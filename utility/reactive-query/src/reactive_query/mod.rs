use crate::*;

mod self_contain;
pub use self_contain::*;

mod dyn_impl;
pub use dyn_impl::*;

mod operator;
pub use operator::*;

pub enum ReactiveQueryRequest {
  MemoryShrinkToFit,
}

pub trait ReactiveQuery: Sync + Send + 'static {
  type Key: CKey;
  type Value: CValue;
  type Changes: Query<Key = Self::Key, Value = ValueChange<Self::Value>>;
  type View: Query<Key = Self::Key, Value = Self::Value>;

  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View);

  fn request(&mut self, request: &mut ReactiveQueryRequest);
}

#[derive(Clone)]
pub struct QueryPreviousView<C, D> {
  current: C,
  delta: D,
}
pub fn make_previous<C, D>(current: C, delta: D) -> QueryPreviousView<C, D> {
  QueryPreviousView { current, delta }
}

/// the impl access the previous V
impl<C, D, K, V> Query for QueryPreviousView<C, D>
where
  C: Query<Key = K, Value = V>,
  D: Query<Key = K, Value = ValueChange<V>>,
  K: CKey,
  V: CValue,
{
  type Key = K;
  type Value = V;
  fn iter_key_value(&self) -> impl Iterator<Item = (K, V)> + '_ {
    let current_not_changed = self
      .current
      .iter_key_value()
      .filter(|(k, _)| !self.delta.contains(k));

    let current_changed = self
      .delta
      .iter_key_value()
      .filter_map(|(k, v)| v.old_value().map(|v| (k, v.clone())));
    current_not_changed.chain(current_changed)
  }

  fn access(&self, key: &K) -> Option<V> {
    if let Some(change) = self.delta.access(key) {
      change.old_value().cloned()
    } else {
      self.current.access_dyn(key)
    }
  }
}

impl<K: CKey, V: CValue> ReactiveQuery for EmptyQuery<K, V> {
  type Key = K;
  type Value = V;
  type Changes = EmptyQuery<K, ValueChange<V>>;
  type View = EmptyQuery<K, V>;
  fn poll_changes(&self, _: &mut Context) -> (Self::Changes, Self::View) {
    (Default::default(), Default::default())
  }
  fn request(&mut self, _: &mut ReactiveQueryRequest) {}
}

pub struct IdenticalChangingCollection<T, V> {
  value: V,
  size: RwLock<T>, // todo, we should use signal like trait
  previous_size: RwLock<Option<u32>>,
}

impl<T, V> IdenticalChangingCollection<T, V> {
  pub fn new(value: V, size: T) -> Self {
    Self {
      value,
      size: RwLock::new(size),
      previous_size: RwLock::new(None),
    }
  }
}

impl<T, V> ReactiveQuery for IdenticalChangingCollection<T, V>
where
  T: Stream<Item = u32> + Send + Sync + Unpin + 'static,
  V: CValue,
{
  type Key = u32;
  type Value = V;
  type Changes = IdenticalDeltaCollection<Self::Value>;
  type View = IdenticalCollection<Self::Value>;

  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View) {
    let new_size = self.size.write().poll_next_unpin(cx);
    let previous_size = self.previous_size.read().unwrap_or(0);
    let new_size = match new_size {
      Poll::Ready(new_size) => new_size.unwrap_or(0),
      Poll::Pending => previous_size,
    };

    let delta = IdenticalDeltaCollection {
      value: self.value.clone(),
      previous_size,
      new_size,
    };

    let current = IdenticalCollection {
      value: self.value.clone(),
      size: new_size,
    };

    *self.previous_size.write() = Some(new_size);

    (delta, current)
  }

  fn request(&mut self, _: &mut ReactiveQueryRequest) {}
}

#[derive(Clone)]
pub struct IdenticalDeltaCollection<V> {
  pub value: V,
  pub previous_size: u32,
  pub new_size: u32,
}

impl<V: CValue> IdenticalDeltaCollection<V> {
  pub fn get_change_value(&self) -> ValueChange<V> {
    if self.new_size > self.previous_size {
      ValueChange::Delta(self.value.clone(), None)
    } else {
      ValueChange::Remove(self.value.clone())
    }
  }
}

impl<V: CValue> Query for IdenticalDeltaCollection<V> {
  type Key = u32;
  type Value = ValueChange<V>;

  fn iter_key_value(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    let range = if self.new_size > self.previous_size {
      self.previous_size..self.new_size
    } else {
      self.new_size..self.previous_size
    };
    let value = self.get_change_value();
    range.map(move |k| (k, value.clone()))
  }

  fn access(&self, key: &Self::Key) -> Option<Self::Value> {
    if key >= &self.previous_size && key < &self.new_size {
      Some(self.get_change_value())
    } else {
      None
    }
  }
}
