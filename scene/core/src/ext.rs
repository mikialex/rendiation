use std::{
  any::{Any, TypeId},
  fmt::Debug,
};

use incremental::{AnyClone, DynIncremental, SimpleIncremental};
use smallvec::SmallVec;

/// like any map, but clone able
#[derive(Default, Clone)]
pub struct DynamicExtension {
  inner: SmallVec<[(std::any::TypeId, Box<dyn DynamicAnyCloneIncremental>); 2]>,
}

pub trait DynamicAnyCloneIncremental: DynIncremental + AnyClone {}
dyn_clone::clone_trait_object!(DynamicAnyCloneIncremental);
impl<T> DynamicAnyCloneIncremental for T where T: DynIncremental + AnyClone {}

#[derive(Clone)]
pub enum DynamicExtensionDelta {
  Insert(Box<dyn DynamicAnyCloneIncremental>),
  Remove(std::any::TypeId),
  Mutate {
    id: std::any::TypeId,
    sub_delta: Box<dyn AnyClone>,
  },
}

impl SimpleIncremental for DynamicExtension {
  type Delta = DynamicExtensionDelta;

  fn s_apply(&mut self, d: Self::Delta) {
    match d {
      DynamicExtensionDelta::Insert(v) => self.insert_dyn(v),
      DynamicExtensionDelta::Remove(id) => {
        self.remove_dyn(id);
      }
      DynamicExtensionDelta::Mutate { id, sub_delta } => {
        self.get_dyn_mut(id).unwrap().apply_dyn(sub_delta).ok();
      }
    }
  }

  fn s_expand(&self, mut f: impl FnMut(Self::Delta)) {
    self
      .inner
      .iter()
      .map(|(_, v)| v)
      .cloned()
      .for_each(|v| f(DynamicExtensionDelta::Insert(v)))
  }
}

impl Debug for DynamicExtension {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("DynamicExtension").finish()
  }
}

impl DynamicExtension {
  #[allow(clippy::borrowed_box)]
  pub fn get_dyn(&self, i: TypeId) -> Option<&Box<dyn DynamicAnyCloneIncremental>> {
    self.inner.iter().find(|(id, _)| *id == i).map(|(_, v)| v)
  }
  pub fn get<T: Any>(&self) -> Option<&T> {
    self
      .get_dyn(TypeId::of::<T>())
      .map(|r| r.as_ref().as_any().downcast_ref::<T>().unwrap())
  }

  pub fn get_dyn_mut(&mut self, i: TypeId) -> Option<&mut Box<dyn DynamicAnyCloneIncremental>> {
    self
      .inner
      .iter_mut()
      .find(|(id, _)| *id == i)
      .map(|(_, v)| v)
  }
  pub fn get_mut<T: Any>(&mut self) -> Option<&mut T> {
    self
      .get_dyn_mut(TypeId::of::<T>())
      .map(|r| r.as_mut().as_any_mut().downcast_mut::<T>().unwrap())
  }

  pub fn insert_dyn(&mut self, item: Box<dyn DynamicAnyCloneIncremental>) {
    let id = item.as_ref().as_any().type_id();
    if let Some(v) = self.get_dyn_mut(id) {
      *v = item
    } else {
      self.inner.push((id, item));
    }
  }

  /// return if removed something
  pub fn remove_dyn(&mut self, i: TypeId) -> bool {
    self
      .inner
      .iter()
      .position(|(id, _)| *id == i)
      .map(|index| {
        self.inner.swap_remove(index);
      })
      .is_some()
  }

  pub fn insert<T: DynamicAnyCloneIncremental>(&mut self, item: T) {
    if let Some(inserted) = self.get_mut() {
      *inserted = item;
    } else {
      self.inner.push((
        TypeId::of::<T>(),
        Box::new(item) as Box<dyn DynamicAnyCloneIncremental>,
      ));
    }
  }

  pub fn with_insert<T: DynamicAnyCloneIncremental>(mut self, item: T) -> Self {
    self.insert(item);
    self
  }

  pub fn with_insert_default<T: DynamicAnyCloneIncremental + Default>(self) -> Self {
    self.with_insert(T::default())
  }
}

#[test]
fn any_clone_downcast() {
  let a = 1_u32;
  let a = Box::new(a) as Box<dyn AnyClone>;
  a.as_ref().as_any().downcast_ref::<u32>().unwrap();
}
