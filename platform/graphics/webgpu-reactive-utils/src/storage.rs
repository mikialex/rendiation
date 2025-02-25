use rendiation_shader_api::{bytes_of, Pod};

use crate::*;

pub type CommonStorageBufferImpl<T> =
  GrowableDirectQueueUpdateBuffer<StorageBufferReadonlyDataView<[T]>>;

/// group of(Rxc<id, T fieldChange>) =maintain=> storage buffer <T>
pub struct ReactiveStorageBufferContainer<T: Std430> {
  pub inner: MultiUpdateContainer<CommonStorageBufferImpl<T>>,
}

fn make_init_size<T: Std430>(size: usize) -> StorageBufferInit<'static, [T]> {
  let bytes = size * std::mem::size_of::<T>();
  let bytes = std::num::NonZeroU64::new(bytes as u64).unwrap();
  StorageBufferInit::<[T]>::Zeroed(bytes)
}

impl<T: Std430> ReactiveStorageBufferContainer<T> {
  pub fn new(gpu_ctx: &GPU) -> Self {
    let init_capacity = 128;
    let data =
      StorageBufferReadonlyDataView::create_by(&gpu_ctx.device, make_init_size(init_capacity));
    let data = create_growable_buffer(gpu_ctx, data, u32::MAX);

    let inner = MultiUpdateContainer::new(data);

    Self { inner }
  }

  pub fn poll_update(&mut self, cx: &mut Context) -> StorageBufferReadonlyDataView<[T]> {
    self.inner.poll_update(cx);
    self.inner.target.gpu().clone()
  }

  pub fn with_source<C>(mut self, source: C, field_offset: usize) -> Self
  where
    C: ReactiveQuery,
    C::Key: LinearIdentified,
    C::Value: Pod,
  {
    let updater = QueryBasedStorageBufferUpdate {
      field_offset: field_offset as u32,
      upstream: source,
    };

    self.inner.add_source(updater);
    self
  }
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

impl<T: Std430> BindableResourceProvider for ReactiveStorageBufferContainer<T> {
  fn get_bindable(&self) -> BindingResourceOwned {
    self.inner.gpu().get_bindable()
  }
}
impl<T: Std430> CacheAbleBindingSource for ReactiveStorageBufferContainer<T> {
  fn get_binding_build_source(&self) -> CacheAbleBindingBuildSource {
    self.inner.gpu().get_binding_build_source()
  }
}
impl<T: Std430> BindableResourceView for ReactiveStorageBufferContainer<T> {
  fn as_bindable(&self) -> rendiation_webgpu::BindingResource {
    self.inner.gpu().as_bindable()
  }
}
