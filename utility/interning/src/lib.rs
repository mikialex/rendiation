use std::hash::Hash;
use std::marker::PhantomData;

use fast_hash_collection::*;

pub struct ValueInterning<T> {
  inner: FastHashMap<T, usize>,
  values: Vec<T>,
}

impl<T> Default for ValueInterning<T> {
  fn default() -> Self {
    Self {
      inner: Default::default(),
      values: Default::default(),
    }
  }
}

#[derive(Debug, Hash, PartialEq, Eq)]
pub struct InternedId<T> {
  value: usize,
  ty: PhantomData<T>,
}

impl<T> Clone for InternedId<T> {
  fn clone(&self) -> Self {
    *self
  }
}

impl<T> Copy for InternedId<T> {}

impl<T> ValueInterning<T>
where
  T: Eq + Hash + Clone,
{
  pub fn compute_intern_id(&mut self, v: &T) -> InternedId<T> {
    let id = self.inner.raw_entry_mut().from_key(v).or_insert_with(|| {
      self.values.push(v.clone());
      let count = self.values.len();
      (v.clone(), count)
    });
    InternedId {
      value: *id.1,
      ty: PhantomData,
    }
  }

  pub fn get_value(&self, id: InternedId<T>) -> Option<&T> {
    self.values.get(id.value)
  }
}
