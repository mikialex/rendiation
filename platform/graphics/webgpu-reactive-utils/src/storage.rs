use rendiation_shader_api::{bytes_of, Pod};

use crate::*;

pub type CommonStorageBufferImpl<T> =
  GrowableDirectQueueUpdateBuffer<StorageBufferReadonlyDataView<[T]>>;

/// group of(Rxc<id, T fieldChange>) =maintain=> storage buffer <T>
pub type ReactiveStorageBufferContainer<T> = MultiUpdateContainer<CommonStorageBufferImpl<T>>;

pub fn create_common_storage_buffer_container<T: Std430>(
  init_capacity_item_count: u32,
  max_item_count: u32,
  gpu_ctx: &GPU,
) -> CommonStorageBufferImpl<T> {
  let init = make_init_size(init_capacity_item_count);
  let data = StorageBufferReadonlyDataView::create_by(&gpu_ctx.device, init);
  create_growable_buffer(gpu_ctx, data, max_item_count)
}

pub fn create_reactive_storage_buffer_container<T: Std430>(
  init_capacity_item_count: u32,
  max_item_count: u32,
  gpu_ctx: &GPU,
) -> ReactiveStorageBufferContainer<T> {
  MultiUpdateContainer::new(create_common_storage_buffer_container(
    init_capacity_item_count,
    max_item_count,
    gpu_ctx,
  ))
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

pub struct QueryBasedStorageBufferUpdate<T> {
  pub field_offset: u32,
  pub upstream: T,
}

impl<T, C> QueryBasedUpdate<T> for QueryBasedStorageBufferUpdate<C>
where
  T: LinearStorageDirectAccess,
  C: ReactiveQuery,
  C::Key: LinearIdentified,
  C::Value: Pod,
{
  fn update_target(&mut self, target: &mut T, cx: &mut Context) {
    let (changes, _) = self.upstream.describe(cx).resolve_kept();
    for (k, v) in changes.iter_key_value() {
      let index = k.alloc_index();

      match v {
        ValueChange::Delta(v, _) => unsafe {
          target
            .set_value_sub_bytes(index, self.field_offset as usize, bytes_of(&v))
            .unwrap();
        },
        ValueChange::Remove(_) => {
          // we could do clear in debug mode
        }
      }
    }
  }
}

pub trait StorageQueryUpdateExt: Sized {
  fn into_query_update_storage(self, field_offset: usize) -> QueryBasedStorageBufferUpdate<Self>;
}
impl<T> StorageQueryUpdateExt for T
where
  T: ReactiveQuery,
{
  fn into_query_update_storage(self, field_offset: usize) -> QueryBasedStorageBufferUpdate<Self> {
    QueryBasedStorageBufferUpdate {
      field_offset: field_offset as u32,
      upstream: self,
    }
  }
}

pub trait StorageQueryResultCtxExt {
  fn take_storage_array_buffer<T: Std430>(
    &mut self,
    token: QueryToken,
  ) -> Option<StorageBufferReadonlyDataView<[T]>>;
}

impl StorageQueryResultCtxExt for QueryResultCtx {
  fn take_storage_array_buffer<T: Std430>(
    &mut self,
    token: QueryToken,
  ) -> Option<StorageBufferReadonlyDataView<[T]>> {
    self
      .take_multi_updater_updated::<CommonStorageBufferImpl<T>>(token)?
      .inner
      .gpu()
      .clone()
      .into()
  }
}
