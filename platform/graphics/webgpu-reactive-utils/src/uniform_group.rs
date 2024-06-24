use fast_hash_collection::FastHashMap;
use rendiation_shader_api::Std140;

use crate::*;

pub type UniformUpdateContainer<K, V> =
  MultiUpdateContainer<FastHashMap<K, UniformBufferDataView<V>>>;

/// group of(Rxc<id, T fieldChange>) =maintain=> group of(uniform buffer <T>)
pub struct UniformCollectionUpdate<T, K, V> {
  field_offset: u32,
  upstream: T,
  phantom: PhantomData<(K, V)>,
  gpu_ctx: GPUResourceCtx,
}

pub trait UniformCollectionUpdateExt<K, V>: Sized {
  fn into_uniform_collection_update(
    self,
    field_offset: usize,
    gpu_ctx: &GPUResourceCtx,
  ) -> UniformCollectionUpdate<Self, K, V>;
}
impl<K, V, T> UniformCollectionUpdateExt<K, V> for T
where
  T: ReactiveCollection<K, V>,
  K: CKey,
  V: CValue,
{
  fn into_uniform_collection_update(
    self,
    field_offset: usize,
    gpu_ctx: &GPUResourceCtx,
  ) -> UniformCollectionUpdate<Self, K, V> {
    UniformCollectionUpdate {
      field_offset: field_offset as u32,
      upstream: self,
      phantom: PhantomData,
      gpu_ctx: gpu_ctx.clone(),
    }
  }
}

impl<T, C, K, V> CollectionUpdate<FastHashMap<K, UniformBufferDataView<T>>>
  for UniformCollectionUpdate<C, K, V>
where
  T: Std140 + Default,
  V: CValue + Pod,
  K: CKey,
  C: ReactiveCollection<K, V>,
{
  fn update_target(
    &mut self,
    target: &mut FastHashMap<K, UniformBufferDataView<T>>,
    cx: &mut Context,
  ) {
    let (changes, _) = self.upstream.poll_changes(cx);
    for (k, v) in changes.iter_key_value() {
      let index = k;

      match v {
        ValueChange::Delta(v, _) => {
          let buffer = target
            .entry(index)
            .or_insert_with(|| UniformBufferDataView::create_default(&self.gpu_ctx.device));

          // here we should do sophisticated optimization to merge the adjacent writes.
          buffer.write_at(&self.gpu_ctx.queue, &v, self.field_offset as u64);
        }
        ValueChange::Remove(_) => {
          target.remove(&index);
        }
      }
    }
  }
}

// rxc<k, v> to map<k, target>
// rxc<k, (fk, v)> to map<fk, target>
// rxc<k, v> to target
