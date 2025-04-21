use std::fmt::Display;

use crate::*;

pub struct EntityHandle<T> {
  pub(crate) ty: PhantomData<T>,
  pub(crate) handle: RawEntityHandle,
}

impl<T> Display for EntityHandle<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.handle)
  }
}

impl<T> LinearIdentified for EntityHandle<T> {
  fn alloc_index(&self) -> u32 {
    self.handle.alloc_index()
  }
}

impl<T> EntityHandle<T> {
  /// # Safety
  ///
  /// handle must be semantically correct as the T entity handle
  pub unsafe fn from_raw(handle: RawEntityHandle) -> Self {
    Self {
      ty: PhantomData,
      handle,
    }
  }
  pub fn into_raw(self) -> RawEntityHandle {
    self.handle
  }
  pub fn some_handle(&self) -> Option<RawEntityHandle> {
    Some(self.handle)
  }
}

impl<T> Copy for EntityHandle<T> {}

impl<T> Clone for EntityHandle<T> {
  fn clone(&self) -> Self {
    *self
  }
}
impl<T> PartialEq for EntityHandle<T> {
  fn eq(&self, other: &Self) -> bool {
    self.handle == other.handle
  }
}
impl<T> Eq for EntityHandle<T> {}
impl<T> Hash for EntityHandle<T> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.ty.hash(state);
    self.handle.hash(state);
  }
}
impl<T> std::fmt::Debug for EntityHandle<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("EntityHandle")
      .field("ty", &self.ty)
      .field("handle", &self.handle)
      .finish()
  }
}

#[repr(transparent)]
#[derive(Serialize, Deserialize)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Facet, Zeroable, Pod)]
pub struct RawEntityHandle(pub(crate) Handle<()>);

impl Display for RawEntityHandle {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl std::fmt::Debug for RawEntityHandle {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let inner = self.0.into_raw_parts();
    f.debug_struct("RawEntityHandle")
      .field("index", &inner.0)
      .field("gen", &inner.1)
      .finish()
  }
}

impl LinearIdentified for RawEntityHandle {
  fn alloc_index(&self) -> u32 {
    self.0.index() as u32
  }
}

impl RawEntityHandle {
  pub fn index(&self) -> u32 {
    self.0.index() as u32
  }
}

impl<T> From<EntityHandle<T>> for RawEntityHandle {
  fn from(val: EntityHandle<T>) -> Self {
    val.handle
  }
}
