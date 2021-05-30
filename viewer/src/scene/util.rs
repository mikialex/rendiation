use std::hash::Hash;
use std::{collections::HashMap, marker::PhantomData};

pub struct ValueIDGenerator<T> {
  inner: HashMap<T, usize>,
}

impl<T> Default for ValueIDGenerator<T> {
  fn default() -> Self {
    Self {
      inner: HashMap::default(),
    }
  }
}

pub struct ValueID<T> {
  value: usize,
  ty: PhantomData<T>,
}

impl<T> ValueIDGenerator<T>
where
  T: Eq + Hash,
{
  pub fn get_uuid(&mut self, v: T) -> ValueID<T> {
    let count = self.inner.len();
    let id = self.inner.entry(v).or_insert(count);
    ValueID {
      value: *id,
      ty: PhantomData,
    }
  }
}
