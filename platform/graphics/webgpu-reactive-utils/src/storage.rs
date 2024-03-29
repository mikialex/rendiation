use rendiation_shader_api::{bytes_of, Pod};

use crate::*;

/// group of(Rxc<id, T fieldChange>) =maintain=> storage buffer <T>
pub struct ReactiveStorageBufferContainer<T: Std430> {
  inner: MultiUpdateContainer<StorageBufferReadOnlyDataView<[T]>>,
  current_size: u32,
  max_idx: Box<dyn Stream<Item = u32> + Unpin>,
  gpu_ctx: GPUResourceCtx,
}

fn make_init_size<T: Std430>(size: usize) -> StorageBufferInit<'static, [T]> {
  let bytes = size * std::mem::size_of::<T>();
  let bytes = std::num::NonZeroU64::new(bytes as u64).unwrap();
  StorageBufferInit::<[T]>::Zeroed(bytes)
}

impl<T: Std430> ReactiveStorageBufferContainer<T> {
  pub fn new(gpu_ctx: GPUResourceCtx, max: impl Stream<Item = u32> + Unpin + 'static) -> Self {
    let init_capacity = 128;
    let data =
      StorageBufferReadOnlyDataView::create_by(&gpu_ctx.device, make_init_size(init_capacity));

    let inner = MultiUpdateContainer::new(data);

    Self {
      inner,
      current_size: init_capacity as u32,
      max_idx: Box::new(max),
      gpu_ctx,
    }
  }

  pub fn poll_update(&mut self, cx: &mut Context) {
    if let Poll::Ready(Some(max_idx)) = self.max_idx.poll_next_unpin(cx) {
      // resize target
      if max_idx > self.current_size {
        let previous_size = self.current_size;
        self.current_size = max_idx;
        let device = &self.gpu_ctx.device;
        let init = make_init_size::<T>(max_idx as usize);
        let new_buffer = StorageBufferReadOnlyDataView::create_by(device, init);
        let old_buffer = std::mem::replace(&mut self.inner.target, new_buffer);
        let new_buffer = &self.inner.target;

        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(
          old_buffer.buffer.gpu(),
          0,
          new_buffer.buffer.gpu(),
          0,
          (previous_size as usize * std::mem::size_of::<T>()) as u64,
        );
        self.gpu_ctx.queue.submit(Some(encoder.finish()));
      }
    }
    self.inner.poll_update(cx);
  }

  pub fn add_source<K: CKey + LinearIdentification, V: CValue + Pod>(
    &mut self,
    source: impl ReactiveCollection<K, V>,
    field_offset: usize,
  ) {
    let updater = CollectionToStorageBufferUpdater {
      field_offset: field_offset as u32,
      stride: std::mem::size_of::<T>() as u32,
      upstream: source,
      phantom: PhantomData,
      gpu_ctx: self.gpu_ctx.clone(),
    };

    self.inner.add_source(Box::new(updater));
  }
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
