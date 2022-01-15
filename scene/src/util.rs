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

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct ValueID<T> {
  value: usize,
  ty: PhantomData<T>,
}

impl<T> Clone for ValueID<T> {
  fn clone(&self) -> Self {
    Self {
      value: self.value,
      ty: self.ty,
    }
  }
}

impl<T> Copy for ValueID<T> {}

impl<T> ValueIDGenerator<T>
where
  T: Eq + Hash + Clone,
{
  pub fn get_uuid(&mut self, v: &T) -> ValueID<T> {
    let count = self.inner.len();
    let id = self
      .inner
      .raw_entry_mut()
      .from_key(v)
      .or_insert_with(|| (v.clone(), count));
    ValueID {
      value: *id.1,
      ty: PhantomData,
    }
  }
}
