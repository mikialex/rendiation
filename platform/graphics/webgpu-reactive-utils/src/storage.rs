use rendiation_shader_api::{bytes_of, Pod};

use crate::*;

pub type CommonStorageBufferImpl<T> =
  GrowableDirectQueueUpdateBuffer<StorageBufferReadonlyDataView<[T]>>;

/// group of(Rxc<id, T fieldChange>) =maintain=> storage buffer <T>
pub type ReactiveStorageBufferContainer<T> = MultiUpdateContainer<CommonStorageBufferImpl<T>>;

pub fn create_reactive_storage_buffer_container<T: Std430>(
  gpu_ctx: &GPU,
) -> ReactiveStorageBufferContainer<T> {
  let init_capacity = 128;
  let data =
    StorageBufferReadonlyDataView::create_by(&gpu_ctx.device, make_init_size(init_capacity));
  let data = create_growable_buffer(gpu_ctx, data, u32::MAX);

  MultiUpdateContainer::new(data)
}

fn make_init_size<T: Std430>(size: usize) -> StorageBufferInit<'static, [T]> {
  let bytes = size * std::mem::size_of::<T>();
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
    let (changes, _) = self.upstream.poll_changes(cx);
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
