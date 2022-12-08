use std::{any::TypeId, fmt::Debug};

use incremental::*;

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
      .map(|r| r.as_any().downcast_ref::<T>().unwrap())
  }

  pub fn get_mut<T: Any>(&mut self) -> Option<&mut T> {
    self
      .inner
      .get_mut(&TypeId::of::<T>())
      .map(|r| r.as_any_mut().downcast_mut::<T>().unwrap())
  }

  pub fn insert<T: AnyClone>(&mut self, item: T) {
    self.inner.insert(TypeId::of::<T>(), Box::new(item));
  }
}

impl Incremental for DynamicExtension {
  type Delta = ();

  type Error = ();

  type Mutator<'a> = SimpleMutator<'a, Self>
  where
    Self: 'a;

  fn create_mutator<'a>(&'a mut self, _: &'a mut dyn FnMut(Self::Delta)) -> Self::Mutator<'a> {
    todo!()
  }

  fn apply(&mut self, _delta: Self::Delta) -> Result<(), Self::Error> {
    todo!()
  }

  fn expand(&self, _cb: impl FnMut(Self::Delta)) {
    todo!()
  }
}
