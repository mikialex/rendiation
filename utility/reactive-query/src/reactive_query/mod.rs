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
  type Compute: ReactiveQueryCompute<Key = Self::Key, Value = Self::Value>;

  fn poll_changes(&self, cx: &mut Context) -> Self::Compute;

  fn request(&mut self, request: &mut ReactiveQueryRequest);
}

pub trait ReactiveQueryCompute {
  type Key: CKey;
  type Value: CValue;
  type Changes: Query<Key = Self::Key, Value = ValueChange<Self::Value>> + 'static;
  type View: Query<Key = Self::Key, Value = Self::Value> + 'static;

  fn resolve(&self) -> (Self::Changes, Self::View);
}

impl<K, V, Change, View> ReactiveQueryCompute for (Change, View)
where
  K: CKey,
  V: CValue,
  Change: Query<Key = K, Value = ValueChange<V>> + 'static,
  View: Query<Key = K, Value = V> + 'static,
{
  type Key = K;
  type Value = V;
  type Changes = Change;
  type View = View;
  fn resolve(&self) -> (Self::Changes, Self::View) {
    (self.0.clone(), self.1.clone())
  }
}

impl<K: CKey, V: CValue> ReactiveQuery for EmptyQuery<K, V> {
  type Key = K;
  type Value = V;
  type Compute = (EmptyQuery<K, ValueChange<V>>, EmptyQuery<K, V>);
  fn poll_changes(&self, _: &mut Context) -> Self::Compute {
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
  type Compute = (IdenticalDeltaCollection<V>, IdenticalCollection<V>);

  fn poll_changes(&self, cx: &mut Context) -> Self::Compute {
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
