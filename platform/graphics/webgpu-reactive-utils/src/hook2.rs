use ::hook::*;
use database::*;

use crate::*;

pub type UniformBufferCollectionRaw<K, T> = FastHashMap<K, UniformBufferDataView<T>>;
pub type UniformBufferCollection<K, T> = Arc<RwLock<FastHashMap<K, UniformBufferDataView<T>>>>;

pub trait DataChangeGPUExt {
  fn update_uniforms<K, V: Std140>(&self, uniforms: &UniformBufferCollection<K, V>, offset: usize);
}

impl<T> DataChangeGPUExt for T {
  fn update_uniforms<K, V: Std140>(&self, uniforms: &UniformBufferCollection<K, V>, offset: usize) {
    todo!()
  }
}

impl<K, V: Std140 + 'static> CanCleanUpFrom<ReactiveQueryCtx> for UniformBufferCollection<K, V> {
  fn drop_from_cx(&mut self, _cx: &mut ReactiveQueryCtx) {}
}
