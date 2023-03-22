use std::{any::TypeId, fmt::Debug};

use crate::*;

/// like any map, but clone able
#[derive(Default, Clone)]
pub struct DynamicExtension {
  inner: HashMap<std::any::TypeId, Box<dyn AnyClone>>,
}

impl Debug for DynamicExtension {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("DynamicExtension").finish()
  }
}

impl DynamicExtension {
  pub fn get<T: Any>(&self) -> Option<&T> {
    self
      .inner
      .get(&TypeId::of::<T>())
      .map(|r| r.as_ref().as_any().downcast_ref::<T>().unwrap())
  }

  pub fn get_mut<T: Any>(&mut self) -> Option<&mut T> {
    self
      .inner
      .get_mut(&TypeId::of::<T>())
      .map(|r| r.as_mut().as_any_mut().downcast_mut::<T>().unwrap())
  }

  pub fn insert<T: AnyClone>(&mut self, item: T) {
    self.inner.insert(TypeId::of::<T>(), Box::new(item));
  }
}

impl SimpleIncremental for DynamicExtension {
  type Delta = ();

  fn s_apply(&mut self, _: Self::Delta) {}

  fn s_expand(&self, _: impl FnMut(Self::Delta)) {}
}
