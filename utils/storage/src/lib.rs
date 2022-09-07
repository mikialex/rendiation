use std::marker::PhantomData;

pub mod generational;
pub mod simple;

/// Generic data container
///
/// Why not we just directly use the S (underlayer container)?.
/// Storage is to wrap any collection and abstract over collection's
/// storage behavior. Using Storage instead of original type, you
/// could switch the underlayer collection easily, because the storage
/// behavior is covered by trait method. The traits provide a unified
/// semantic to handle collection relationship.
pub struct Storage<T, S: StorageBehavior<T>> {
  data: S,
  phantom: PhantomData<T>,
}

pub type Handle<T, S> = <S as StorageBehavior<T>>::Handle;

pub trait StorageBehavior<T>: Sized + Default {
  type Handle: Copy;

  fn insert(&mut self, v: T) -> Self::Handle;
  fn remove(&mut self, handle: Self::Handle) -> Option<T>;
  fn get(&self, handle: Self::Handle) -> Option<&T>;
  fn get_mut(&mut self, handle: Self::Handle) -> Option<&mut T>;
  fn size(&self) -> usize;
  fn is_empty(&self) -> bool {
    self.size() == 0
  }
}

impl<T, S: StorageBehavior<T>> Default for Storage<T, S> {
  fn default() -> Self {
    Self {
      data: Default::default(),
      phantom: PhantomData,
    }
  }
}

impl<T, S: StorageBehavior<T>> Storage<T, S> {
  pub fn insert(&mut self, v: T) -> S::Handle {
    S::insert(&mut self.data, v)
  }

  pub fn remove(&mut self, h: S::Handle) -> Option<T> {
    S::remove(&mut self.data, h)
  }

  pub fn get(&self, h: S::Handle) -> Option<&T> {
    S::get(&self.data, h)
  }

  /// # Safety
  ///
  /// Any bound check or underlayer check is skipped
  /// .
  pub unsafe fn get_unchecked(&self, h: S::Handle) -> &T {
    self.get(h).unwrap_unchecked()
  }

  pub fn get_mut(&mut self, h: S::Handle) -> Option<&mut T> {
    S::get_mut(&mut self.data, h)
  }

  /// # Safety
  ///
  /// Any bound check or underlayer check is skipped
  /// .
  pub unsafe fn get_mut_unchecked(&mut self, h: S::Handle) -> &mut T {
    self.get_mut(h).unwrap_unchecked()
  }

  pub fn contains(&self, h: S::Handle) -> bool {
    S::get(&self.data, h).is_some()
  }

  pub fn size(&self) -> usize {
    S::size(&self.data)
  }
}

pub trait NoneOverlappingStorage<T>: StorageBehavior<T> {
  fn get_mut_pair(&mut self, handle: (Self::Handle, Self::Handle)) -> Option<(&mut T, &mut T)>;
}

impl<T, S: NoneOverlappingStorage<T>> Storage<T, S> {
  pub fn get_mut_pair(&mut self, handle: (S::Handle, S::Handle)) -> Option<(&mut T, &mut T)> {
    S::get_mut_pair(&mut self.data, (handle.0, handle.1))
  }

  /// # Safety
  ///
  /// Any bound check or underlayer check is skipped
  /// .
  pub unsafe fn get_mut_pair_unchecked(
    &mut self,
    handle: (S::Handle, S::Handle),
  ) -> (&mut T, &mut T) {
    S::get_mut_pair(&mut self.data, (handle.0, handle.1)).unwrap_unchecked()
  }
}

pub trait HandlePredictableStorage<T>: StorageBehavior<T> {
  fn insert_with(&mut self, creator: impl FnOnce(Self::Handle) -> T) -> Self::Handle;
}
impl<T, S: HandlePredictableStorage<T>> Storage<T, S> {
  pub fn insert_with(&mut self, creator: impl FnOnce(S::Handle) -> T) -> S::Handle {
    S::insert_with(&mut self.data, creator)
  }
}
