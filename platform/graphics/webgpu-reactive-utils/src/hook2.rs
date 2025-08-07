use ::hook::*;

use crate::*;

pub type UniformBufferCollectionRaw<K, T> = FastHashMap<K, UniformBufferDataView<T>>;
pub type UniformBufferCollection<K, T> = Arc<RwLock<FastHashMap<K, UniformBufferDataView<T>>>>;

pub trait DataChangeGPUExt {
  fn update_uniforms<K: LinearIdentification + CKey, U: Std140 + Default>(
    &self,
    uniforms: &UniformBufferCollection<K, U>,
    offset: usize,
    gpu: &GPU,
  );
}

impl<T, X> DataChangeGPUExt for Option<X>
where
  T: Pod,
  X: DataChanges<Key = u32, Value = T>,
{
  fn update_uniforms<K: LinearIdentification + CKey, U: Std140 + Default>(
    &self,
    uniforms: &UniformBufferCollection<K, U>,
    offset: usize,
    gpu: &GPU,
  ) {
    if let Some(change) = self {
      if change.has_change() {
        let mut uniform = uniforms.write();
        for id in change.iter_removed() {
          uniform.remove(&K::from_alloc_index(id));
        }

        for (id, value) in change.iter_update_or_insert() {
          let buffer = uniform
            .entry(K::from_alloc_index(id))
            .or_insert_with(|| UniformBufferDataView::create_default(&gpu.device));
          // todo, here we should do sophisticated optimization to merge the adjacent writes.
          buffer.write_at(&gpu.queue, &value, offset as u64);
        }
      }
    }
  }
}

impl<K, V: Std140 + 'static> CanCleanUpFrom<QueryGPUHookDropCx<'_>>
  for UniformBufferCollection<K, V>
{
  fn drop_from_cx(&mut self, _cx: &mut QueryGPUHookDropCx<'_>) {}
}
