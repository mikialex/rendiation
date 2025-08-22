use crate::*;

pub type CommonStorageBufferImpl<T> =
  GrowableDirectQueueUpdateBuffer<StorageBufferReadonlyDataView<[T]>>;

pub trait GetBufferHelper<T: Std430> {
  fn get_gpu_buffer(&self) -> StorageBufferReadonlyDataView<[T]>;
}

impl<T: Std430> GetBufferHelper<T> for CommonStorageBufferImpl<T> {
  fn get_gpu_buffer(&self) -> StorageBufferReadonlyDataView<[T]> {
    self.inner.gpu().clone()
  }
}

pub fn create_common_storage_buffer_container<T: Std430>(
  init_capacity_item_count: u32,
  max_item_count: u32,
  gpu_ctx: &GPU,
) -> CommonStorageBufferImpl<T> {
  let init = make_init_size(init_capacity_item_count);
  let data = StorageBufferReadonlyDataView::create_by(&gpu_ctx.device, init);
  create_growable_buffer(gpu_ctx, data, max_item_count)
}

pub type CommonStorageBufferImplWithHostBackup<T> =
  VecWithStorageBuffer<CommonStorageBufferImpl<T>>;

pub fn create_common_storage_buffer_with_host_backup_container<T: Std430 + Default>(
  init_capacity_item_count: u32,
  max_item_count: u32,
  gpu_ctx: &GPU,
) -> CommonStorageBufferImplWithHostBackup<T> {
  let init = make_init_size(init_capacity_item_count);
  let data = StorageBufferReadonlyDataView::create_by(&gpu_ctx.device, init);
  create_growable_buffer(gpu_ctx, data, max_item_count).with_vec_backup(T::default(), false)
}

fn make_init_size<T: Std430>(size: u32) -> StorageBufferInit<'static, [T]> {
  let bytes = (size as usize) * std::mem::size_of::<T>();
  let bytes = std::num::NonZeroU64::new(bytes as u64).unwrap();
  StorageBufferInit::<[T]>::Zeroed(bytes)
}
