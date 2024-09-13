mod deduplication;
mod generational;
mod index_kept;
mod index_reuse;
mod item_package;
mod linklist_pool;
mod multi_hash;

pub use deduplication::*;
use fast_hash_collection::*;
pub use generational::*;
pub use index_kept::*;
pub use index_reuse::*;
pub use item_package::*;
pub use linklist_pool::*;
pub use multi_hash::*;
pub type Handle<T> = <T as StorageBehavior>::Handle;

pub trait StorageBehavior: Sized + Default {
  type Item;
  type Handle: Copy;

  fn insert(&mut self, v: Self::Item) -> Self::Handle;
  fn size(&self) -> usize;
  fn is_empty(&self) -> bool {
    self.size() == 0
  }
}

pub trait AccessibleStorage: StorageBehavior {
  fn get(&self, handle: Self::Handle) -> Option<&Self::Item>;
  fn get_mut(&mut self, handle: Self::Handle) -> Option<&mut Self::Item>;
}

pub trait RemoveAbleStorage: StorageBehavior {
  fn remove(&mut self, handle: Self::Handle) -> Option<Self::Item>;
}

pub trait NoneOverlappingStorage: AccessibleStorage {
  fn get_mut_pair(
    &mut self,
    handle: (Self::Handle, Self::Handle),
  ) -> Option<(&mut Self::Item, &mut Self::Item)>;
}

pub trait HandlePredictableStorage: StorageBehavior {
  fn insert_with(&mut self, creator: impl FnOnce(Self::Handle) -> Self::Item) -> Self::Handle;
}

/// this is use for saving memory. u32max-1 should be enough for any container's max size, and
/// Option<u32> could be represent by u32max.
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
