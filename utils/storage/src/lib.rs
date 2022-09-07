use std::marker::PhantomData;

pub mod generational;
pub mod simple;

/// Generic data container
pub struct Storage<T, S: StorageBehavior<T>> {
  data: S,
  phantom: PhantomData<T>,
}
pub struct Handle<T, S: StorageBehavior<T>> {
  phantom: PhantomData<S>,
  phantom_t: PhantomData<T>,
  handle: S::Handle,
}

impl<T, S: StorageBehavior<T>> Clone for Handle<T, S> {
  fn clone(&self) -> Self {
    Self::new(self.handle)
  }
}

impl<T, S: StorageBehavior<T>> Copy for Handle<T, S> {}

impl<T, S: StorageBehavior<T>> Handle<T, S> {
  pub fn new(handle: S::Handle) -> Self {
    Self {
      phantom: PhantomData,
      phantom_t: PhantomData,
      handle,
    }
  }
}

pub trait StorageBehavior<T>: Sized + Default {
  type Handle: Copy;

  fn insert(&mut self, v: T) -> Handle<T, Self>;
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
  pub fn insert(&mut self, v: T) -> Handle<T, S> {
    S::insert(&mut self.data, v)
  }

  pub fn remove(&mut self, h: Handle<T, S>) -> Option<T> {
    S::remove(&mut self.data, h.handle)
  }

  pub fn get(&self, h: Handle<T, S>) -> Option<&T> {
    S::get(&self.data, h.handle)
  }

  /// # Safety
  ///
  /// Any bound check or underlayer check is skipped
  /// .
  pub unsafe fn get_unchecked(&self, h: Handle<T, S>) -> &T {
    self.get(h).unwrap_unchecked()
  }

  pub fn get_mut(&mut self, h: Handle<T, S>) -> Option<&mut T> {
    S::get_mut(&mut self.data, h.handle)
  }

  /// # Safety
  ///
  /// Any bound check or underlayer check is skipped
  /// .
  pub unsafe fn get_mut_unchecked(&mut self, h: Handle<T, S>) -> &mut T {
    self.get_mut(h).unwrap_unchecked()
  }

  pub fn contains(&self, h: Handle<T, S>) -> bool {
    S::get(&self.data, h.handle).is_some()
  }

  pub fn size(&self) -> usize {
    S::size(&self.data)
  }
}

pub trait NoneOverlappingStorage<T>: StorageBehavior<T> {
  fn get_mut_pair(&mut self, handle: (Self::Handle, Self::Handle)) -> Option<(&mut T, &mut T)>;
}

impl<T, S: NoneOverlappingStorage<T>> Storage<T, S> {
  pub fn get_mut_pair(&mut self, handle: (Handle<T, S>, Handle<T, S>)) -> Option<(&mut T, &mut T)> {
    S::get_mut_pair(&mut self.data, (handle.0.handle, handle.1.handle))
  }

  /// # Safety
  ///
  /// Any bound check or underlayer check is skipped
  /// .
  pub unsafe fn get_mut_pair_unchecked(
    &mut self,
    handle: (Handle<T, S>, Handle<T, S>),
  ) -> (&mut T, &mut T) {
    S::get_mut_pair(&mut self.data, (handle.0.handle, handle.1.handle)).unwrap_unchecked()
  }
}

pub trait HandlePredictableStorage<T>: StorageBehavior<T> {
  fn insert_with(&mut self, creator: impl FnOnce(Handle<T, Self>) -> T) -> Handle<T, Self>;
}
impl<T, S: HandlePredictableStorage<T>> Storage<T, S> {
  pub fn insert_with(&mut self, creator: impl FnOnce(Handle<T, S>) -> T) -> Handle<T, S> {
    S::insert_with(&mut self.data, creator)
  }
}
