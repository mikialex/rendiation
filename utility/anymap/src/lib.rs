use std::any::{Any, TypeId};

use fast_hash_collection::FastHashMap;

#[derive(Default)]
pub struct AnyMap {
  map: FastHashMap<TypeId, Box<dyn Any>>,
}

impl AnyMap {
  pub fn register<T: Any>(&mut self, value: T) {
    self.map.insert(TypeId::of::<T>(), Box::new(value));
  }
  pub fn get_mut<T: Any>(&mut self) -> Option<&mut T> {
    self
      .map
      .get_mut(&TypeId::of::<T>())
      .and_then(|x| x.downcast_mut())
  }
  pub fn get<T: Any>(&self) -> Option<&T> {
    self
      .map
      .get(&TypeId::of::<T>())
      .and_then(|x| x.downcast_ref())
  }
  pub fn take<T: Any>(&mut self) -> Option<T> {
    self
      .map
      .remove(&TypeId::of::<T>())
      .and_then(|x| x.downcast().ok().map(|v| *v))
  }
}
