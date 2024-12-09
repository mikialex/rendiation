#[cfg(feature = "enabled")]
use std::sync::RwLock;
use std::{
  any::{Any, TypeId},
  marker::PhantomData,
};

use fast_hash_collection::FastHashMap;
#[cfg(feature = "enabled")]
use once_cell::sync::Lazy;

use crate::*;

#[cfg(feature = "enabled")]
pub static HEAP_TOOL_GLOBAL_INSTANCE_COUNTER: Lazy<RwLock<InstanceCounter>> =
  Lazy::new(|| RwLock::new(Default::default()));

#[derive(Default)]
pub struct InstanceCounter {
  inner: FastHashMap<TypeId, (CounterRecord, &'static str)>,
}

impl InstanceCounter {
  fn get_or_insert_instance_record<T: Any>(&mut self) -> &CounterRecord {
    &self
      .inner
      .entry(TypeId::of::<T>())
      .or_insert_with(|| (Default::default(), std::any::type_name::<T>()))
      .0
  }

  #[allow(unused)]
  fn increase_instance<T: Any>(&mut self) {
    self.get_or_insert_instance_record::<T>().increase(1);
  }

  #[allow(unused)]
  fn decrease_instance<T: Any>(&mut self) {
    self.get_or_insert_instance_record::<T>().decrease(1);
  }

  pub fn reset_instance_history_peak<T: Any>(&mut self) {
    self
      .get_or_insert_instance_record::<T>()
      .reset_history_peak_to_current();
  }
  pub fn reset_all_instance_history_peak(&mut self) {
    for (_, (v, _)) in self.inner.iter_mut() {
      v.reset_history_peak_to_current()
    }
  }

  pub fn report_instance_count<T: Any>(&mut self) -> CounterRecordReport<u64> {
    self.get_or_insert_instance_record::<T>().report()
  }

  pub fn report_all_instance_count(
    &self,
  ) -> impl Iterator<Item = (&'static str, CounterRecordReport<u64>)> + '_ {
    self.inner.iter().map(|(_, (v, name))| (*name, v.report()))
  }
}

/// this struct is to act as the counting point of the type T.
/// user could use this struct in production build as we not enabled the real counting by default
pub struct Counted<T: Any> {
  phantom: PhantomData<T>,
}

/// no matter the enabled feature is active or not, we always keep this trait.
/// adding or removing a drop trait is breaking change
impl<T: Any> Drop for Counted<T> {
  fn drop(&mut self) {
    #[cfg(feature = "enabled")]
    HEAP_TOOL_GLOBAL_INSTANCE_COUNTER
      .write()
      .unwrap()
      .decrease_instance::<T>();
  }
}

impl<T: Any> std::fmt::Debug for Counted<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("instance counter")
  }
}

impl<T: Any> Default for Counted<T> {
  fn default() -> Self {
    #[cfg(feature = "enabled")]
    HEAP_TOOL_GLOBAL_INSTANCE_COUNTER
      .write()
      .unwrap()
      .increase_instance::<T>();
    Self {
      phantom: Default::default(),
    }
  }
}

impl<T: Any> Clone for Counted<T> {
  fn clone(&self) -> Self {
    #[cfg(feature = "enabled")]
    HEAP_TOOL_GLOBAL_INSTANCE_COUNTER
      .write()
      .unwrap()
      .increase_instance::<T>();
    Self {
      phantom: PhantomData,
    }
  }
}
