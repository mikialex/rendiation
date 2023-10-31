mod deduplication;
mod generational;
mod generational_shrinkable;
mod index_kept;
mod index_reuse;
mod linklist_pool;

pub use deduplication::*;
pub use generational::*;
pub use generational_shrinkable::*;
pub use index_kept::*;
pub use index_reuse::*;
pub use linklist_pool::*;

pub type Handle<T, S> = <S as StorageBehavior<T>>::Handle;

pub trait StorageBehavior<T>: Sized + Default {
  type Handle: Copy;

  fn insert(&mut self, v: T) -> Self::Handle;
  fn get(&self, handle: Self::Handle) -> Option<&T>;
  fn get_mut(&mut self, handle: Self::Handle) -> Option<&mut T>;
  fn size(&self) -> usize;
  fn is_empty(&self) -> bool {
    self.size() == 0
  }
}

pub trait RemoveAbleStorage<T>: StorageBehavior<T> {
  fn remove(&mut self, handle: Self::Handle) -> Option<T>;
}

pub trait NoneOverlappingStorage<T>: StorageBehavior<T> {
  fn get_mut_pair(&mut self, handle: (Self::Handle, Self::Handle)) -> Option<(&mut T, &mut T)>;
}

pub trait HandlePredictableStorage<T>: StorageBehavior<T> {
  fn insert_with(&mut self, creator: impl FnOnce(Self::Handle) -> T) -> Self::Handle;
}

/// this is use for saving memory. u32 should be enough for most container size, and Option<u32>
/// could be represent by u32 max.
#[derive(Clone, Copy)]
pub struct IndexPtr {
  index: u32,
}

impl IndexPtr {
  pub fn new(index: Option<usize>) -> Self {
    Self {
      index: index.map(|v| v as u32).unwrap_or(u32::MAX),
    }
  }
  pub fn get(&self) -> Option<usize> {
    (self.index != u32::MAX).then_some(self.index as usize)
  }

  pub fn set(&mut self, index: Option<usize>) {
    *self = Self::new(index)
  }
}
