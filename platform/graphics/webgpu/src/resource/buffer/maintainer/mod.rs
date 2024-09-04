use crate::*;

mod gpu_raw;
mod grow_behavior;
pub use grow_behavior::*;
mod vec_backup;
pub use vec_backup::*;
mod queue_direct_update;
pub use queue_direct_update::*;
mod defer_mutation;
pub use defer_mutation::*;

pub trait GPULinearStorage: Sized {
  type GPUType;
  fn update_gpu(&mut self, encoder: &mut GPUCommandEncoder);
  fn gpu(&self) -> &Self::GPUType;
  fn raw_gpu(&self) -> &GPUBufferResourceView;

  fn with_queue_direct_update(self, queue: &GPUQueue) -> GPUStorageDirectQueueUpdate<Self> {
    GPUStorageDirectQueueUpdate {
      queue: queue.clone(),
      inner: self,
    }
  }
}

pub trait LinearStorageBase: Sized {
  type Item: Copy;
  fn max_size(&self) -> u32;
}

// pub trait LinearStorageShrinkInfo: LinearStorageBase {
//   fn use_item_count(&self) -> u32;
//   fn largest_in_use_idx(&self) -> u32;
// }

pub trait LinearStorage: LinearStorageBase {
  fn remove(&mut self, idx: u32);
  fn removes(&mut self, offset: u32, len: usize) {
    for i in offset..(offset + len as u32) {
      self.remove(i);
    }
  }
  fn set_value(&mut self, idx: u32, v: Self::Item) -> Option<()>;
  fn set_values(&mut self, offset: u32, v: &[Self::Item]) -> Option<()> {
    for i in offset..(offset + v.len() as u32) {
      self.set_value(i, v[i as usize - offset as usize])?;
    }
    Some(())
  }

  fn with_defer_update(self) -> DeferMutationToGPUUpdate<Self> {
    DeferMutationToGPUUpdate {
      inner: self,
      updates: Default::default(),
    }
  }

  fn with_vec_backup(self, none_default: Self::Item, diff: bool) -> VecWithStorageBuffer<Self> {
    VecWithStorageBuffer {
      vec: vec![none_default; self.max_size() as usize],
      inner: self,
      diff,
      none_default,
    }
  }
}

pub trait LinearAllocatorStorage: LinearStorageBase {
  fn remove(&mut self, idx: u32, relocation_handler: &mut dyn FnMut((u32, u32)));
  fn set_value(
    &mut self,
    v: Self::Item,
    relocation_handler: &mut dyn FnMut((u32, u32)),
  ) -> Option<usize>;
}

pub trait RangeAllocatorStorage: LinearStorageBase {
  fn remove(&mut self, idx: u32, relocation_handler: &mut dyn FnMut((u32, u32)));
  fn set_values(
    &mut self,
    v: &[Self::Item],
    relocation_handler: &mut dyn FnMut((u32, u32)),
  ) -> Option<usize>;
}

pub trait ResizeableLinearStorage: LinearStorageBase {
  fn resize(&mut self, new_size: u32);

  fn with_grow_behavior(
    self,
    resizer: impl Fn(ResizeInput) -> Option<u32> + 'static,
  ) -> CustomGrowBehaviorMaintainer<Self> {
    CustomGrowBehaviorMaintainer {
      inner: self,
      size_adjust: Box::new(resizer),
    }
  }
}

pub trait LinearStorageViewAccess: LinearStorageBase {
  fn view(&self) -> &[Self::Item];
  fn get(&self, idx: u32) -> Option<&Self::Item> {
    self.view().get(idx as usize)
  }
}
