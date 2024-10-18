use fast_hash_collection::FastHashMap;
use rendiation_shader_api::Std140;

use crate::*;

pub type UniformUpdateContainer<K, V> =
  MultiUpdateContainer<FastHashMap<K, UniformBufferDataView<V>>>;

/// group of(Rxc<id, T fieldChange>) =maintain=> group of(uniform buffer <T>)
pub struct UniformCollectionUpdate<T> {
  field_offset: u32,
  upstream: T,
  gpu_ctx: GPU,
}

pub trait UniformCollectionUpdateExt: Sized {
  fn into_uniform_collection_update(
    self,
    field_offset: usize,
    gpu_ctx: &GPU,
  ) -> UniformCollectionUpdate<Self>;
}
impl<T> UniformCollectionUpdateExt for T
where
  T: ReactiveCollection,
{
  fn into_uniform_collection_update(
    self,
    field_offset: usize,
    gpu_ctx: &GPU,
  ) -> UniformCollectionUpdate<Self> {
    UniformCollectionUpdate {
      field_offset: field_offset as u32,
      upstream: self,
      gpu_ctx: gpu_ctx.clone(),
    }
  }
}

impl<T, C> CollectionUpdate<FastHashMap<C::Key, UniformBufferDataView<T>>>
  for UniformCollectionUpdate<C>
where
  T: Std140 + Default,
  C: ReactiveCollection,
  C::Value: Pod,
{
  fn update_target(
    &mut self,
    target: &mut FastHashMap<C::Key, UniformBufferDataView<T>>,
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
