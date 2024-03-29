use rendiation_shader_api::{bytes_of, Pod};

use crate::*;

/// group of(Rxc<id, T fieldChange>) =maintain=> storage buffer <T>
pub struct ReactiveStorageBufferContainer<T: Std430> {
  inner: MultiUpdateContainer<StorageBufferReadOnlyDataView<[T]>>,
}

struct CollectionToStorageBufferUpdater<T, K, V> {
  field_offset: u32,
  stride: u32,
  upstream: T,
  phantom: PhantomData<(K, V)>,
  gpu_ctx: GPUResourceCtx,
}

impl<T, C, K, V> CollectionUpdate<StorageBufferReadOnlyDataView<[T]>>
  for CollectionToStorageBufferUpdater<C, K, V>
where
  T: Std430,
  V: CValue + Pod,
  K: CKey + LinearIdentification,
  C: ReactiveCollection<K, V>,
{
  fn update_target(&mut self, target: &mut StorageBufferReadOnlyDataView<[T]>, cx: &mut Context) {
    if let Poll::Ready(changes) = self.upstream.poll_changes(cx) {
      for (k, v) in changes.iter_key_value() {
        let index = k.alloc_index();
        let offset = index * self.stride + self.field_offset;

        match v {
          ValueChange::Delta(v, _) => {
            // here we should do sophisticated optimization to merge the adjacent writes.
            // todo resize
            target.write_at(offset as u64, bytes_of(&v), &self.gpu_ctx.queue);
          }
          ValueChange::Remove(_) => {
            // we could do clear in debug mode
          }
        }
      }
    }
  }
}

impl<T: Std430> BindableResourceProvider for ReactiveStorageBufferContainer<T> {
  fn get_bindable(&self) -> BindingResourceOwned {
    self.inner.get_bindable()
  }
}
impl<T: Std430> CacheAbleBindingSource for ReactiveStorageBufferContainer<T> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    self.inner.get_binding_build_source()
  }
}
impl<T: Std430> BindableResourceView for ReactiveStorageBufferContainer<T> {
  fn as_bindable(&self) -> rendiation_webgpu::BindingResource {
    self.inner.as_bindable()
  }
}
