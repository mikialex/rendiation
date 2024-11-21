use crate::*;

mod gpu_raw;
pub use gpu_raw::*;
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
  type Item: Pod;
  fn max_size(&self) -> u32;
}

// pub trait LinearStorageShrinkInfo: LinearStorageBase {
//   fn use_item_count(&self) -> u32;
//   fn largest_in_use_idx(&self) -> u32;
// }

pub trait LinearStorageDirectAccess: LinearStorageBase {
  fn remove(&mut self, idx: u32) -> Option<()>;
  fn removes(&mut self, offset: u32, len: u32) -> Option<()> {
    for i in offset..(offset + len) {
      self.remove(i)?;
    }
    Some(())
  }
  #[must_use]
  fn set_value(&mut self, idx: u32, v: Self::Item) -> Option<()>;
  #[must_use]
  fn set_values(&mut self, offset: u32, v: &[Self::Item]) -> Option<()> {
    for i in offset..(offset + v.len() as u32) {
      self.set_value(i, v[i as usize - offset as usize])?;
    }
    Some(())
  }
  /// # Safety
  ///
  /// this is a special way to support partial item updates. v must be inbound
  #[must_use]
  unsafe fn set_value_sub_bytes(
    &mut self,
    idx: u32,
    field_byte_offset: usize,
    v: &[u8],
  ) -> Option<()>;

  fn with_defer_update(self) -> DeferMutationToGPUUpdate<Self> {
    DeferMutationToGPUUpdate {
      inner: self,
      updates: Default::default(),
      bump_bytes: Default::default(),
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

pub trait ResizableLinearStorage: LinearStorageBase {
  /// return if resize success
  fn resize(&mut self, new_size: u32) -> bool;

  fn with_grow_behavior(
    self,
    resizer: impl Fn(ResizeInput) -> Option<u32> + 'static + Send + Sync,
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

pub type GrowableDirectQueueUpdateBuffer<T> =
  CustomGrowBehaviorMaintainer<GPUStorageDirectQueueUpdate<ResizableGPUBuffer<T>>>;

pub fn create_growable_buffer<T: GPULinearStorageImpl>(
  gpu: &GPU,
  buffer: T,
  max_size: u32,
) -> GrowableDirectQueueUpdateBuffer<T> {
  ResizableGPUBuffer {
    gpu: buffer,
    ctx: gpu.clone(),
  }
  .with_queue_direct_update(&gpu.queue)
  .with_grow_behavior(
    move |ResizeInput {
            current_size,
            required_size,
          }| {
      if required_size > max_size {
        None
      } else {
        Some((current_size * 2).min(max_size))
      }
    },
  )
}

pub type GrowableHostedDirectQueueUpdateBuffer<T> = CustomGrowBehaviorMaintainer<
  VecWithStorageBuffer<GPUStorageDirectQueueUpdate<ResizableGPUBuffer<T>>>,
>;

pub fn create_growable_buffer_with_host_back<T: GPULinearStorageImpl>(
  gpu: &GPU,
  buffer: T,
  max_size: u32,
  diff_update: bool,
) -> GrowableHostedDirectQueueUpdateBuffer<T> {
  ResizableGPUBuffer {
    gpu: buffer,
    ctx: gpu.clone(),
  }
  .with_queue_direct_update(&gpu.queue)
  .with_vec_backup(T::Item::zeroed(), diff_update)
  .with_grow_behavior(
    move |ResizeInput {
            current_size,
            required_size,
          }| {
      if required_size > max_size {
        None
      } else {
        Some((current_size * 2).min(max_size))
      }
    },
  )
}
