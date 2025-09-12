use crate::*;

pub type CommonStorageBufferImpl<T> =
  GrowableDirectQueueUpdateBuffer<AbstractReadonlyStorageBuffer<[T]>>;

pub trait GetBufferHelper<T: Std430> {
  fn get_gpu_buffer(&self) -> AbstractReadonlyStorageBuffer<[T]>;
}

impl<T: Std430 + ShaderSizedValueNodeType> GetBufferHelper<T> for CommonStorageBufferImpl<T> {
  fn get_gpu_buffer(&self) -> AbstractReadonlyStorageBuffer<[T]> {
    self.inner.gpu().clone()
  }
}

pub fn create_common_storage_buffer_container<T: Std430 + ShaderSizedValueNodeType>(
  label: &str,
  init_capacity_item_count: u32,
  max_item_count: u32,
  allocator: &dyn AbstractStorageAllocator,
  gpu_ctx: &GPU,
) -> CommonStorageBufferImpl<T> {
  let data = allocator.allocate_readonly(
    make_init_size::<T>(init_capacity_item_count),
    &gpu_ctx.device,
    Some(label),
  );
  create_growable_buffer(gpu_ctx, data, max_item_count)
}

pub type CommonStorageBufferImplWithHostBackup<T> =
  VecWithStorageBuffer<CommonStorageBufferImpl<T>>;

pub fn create_common_storage_buffer_with_host_backup_container<T>(
  init_capacity_item_count: u32,
  max_item_count: u32,
  allocator: &dyn AbstractStorageAllocator,
  gpu_ctx: &GPU,
  label: &str,
) -> CommonStorageBufferImplWithHostBackup<T>
where
  T: Std430 + ShaderSizedValueNodeType + Default,
{
  let data = allocator.allocate_readonly(
    make_init_size::<T>(init_capacity_item_count),
    &gpu_ctx.device,
    Some(label),
  );
  create_growable_buffer(gpu_ctx, data, max_item_count).with_vec_backup(T::default(), false)
}

fn make_init_size<T: Std430>(size: u32) -> u64 {
  ((size as usize) * std::mem::size_of::<T>()) as u64
}
