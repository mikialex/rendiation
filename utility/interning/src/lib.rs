use std::hash::Hash;
use std::marker::PhantomData;

use fast_hash_collection::*;

pub struct ValueInterning<T> {
  inner: FastHashMap<T, usize>,
  values: Vec<T>,
}

#[macro_export]
macro_rules! define_static_id_generator {
  ($Name: tt, $Type: ty) => {
    static $Name: once_cell::sync::Lazy<parking_lot::Mutex<ValueInterning<$Type>>> =
      once_cell::sync::Lazy::new(|| parking_lot::Mutex::new(ValueInterning::default()));
  };
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
pub struct InternedValue<T> {
  value: usize,
  ty: PhantomData<T>,
}

impl<T> Clone for InternedValue<T> {
  fn clone(&self) -> Self {
    *self
  }
}

impl<T> Copy for InternedValue<T> {}

impl<T> ValueInterning<T>
where
  T: Eq + Hash + Clone,
{
  pub fn get_uuid(&mut self, v: &T) -> InternedValue<T> {
    let count = self.values.len();
    let id = self.inner.raw_entry_mut().from_key(v).or_insert_with(|| {
      self.values.push(v.clone());
      (v.clone(), count)
    });
    InternedValue {
      value: *id.1,
      ty: PhantomData,
    }
  }

  pub fn get_value(&self, id: InternedValue<T>) -> Option<&T> {
    self.values.get(id.value)
  }
}
