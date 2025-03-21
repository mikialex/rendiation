use fast_hash_collection::FastHashMap;
use rendiation_shader_api::Std140;

use crate::*;

pub type UniformUpdateContainer<K, V> =
  MultiUpdateContainer<FastHashMap<K, UniformBufferDataView<V>>>;

/// group of(Rxc<id, T fieldChange>) =maintain=> group of(uniform buffer <T>)
pub struct QueryBasedUniformUpdate<T> {
  field_offset: u32,
  upstream: T,
  gpu_ctx: GPU,
}

pub trait UniformQueryUpdateExt: Sized {
  fn into_query_update_uniform(
    self,
    field_offset: usize,
    gpu_ctx: &GPU,
  ) -> QueryBasedUniformUpdate<Self>;
}
impl<T> UniformQueryUpdateExt for T
where
  T: ReactiveQuery,
{
  fn into_query_update_uniform(
    self,
    field_offset: usize,
    gpu_ctx: &GPU,
  ) -> QueryBasedUniformUpdate<Self> {
    QueryBasedUniformUpdate {
      field_offset: field_offset as u32,
      upstream: self,
      gpu_ctx: gpu_ctx.clone(),
    }
  }
}

impl<T, C> QueryBasedUpdate<FastHashMap<C::Key, UniformBufferDataView<T>>>
  for QueryBasedUniformUpdate<C>
where
  T: Std140 + Default,
  C: ReactiveQuery,
  C::Value: Pod,
{
  fn update_target(
    &mut self,
    target: &mut FastHashMap<C::Key, UniformBufferDataView<T>>,
    cx: &mut Context,
  ) {
    let (changes, _) = self.upstream.poll_changes(cx).resolve();
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
