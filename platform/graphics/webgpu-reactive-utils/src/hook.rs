use ::hook::*;
use database::{EntityHandle, ForeignKeySemantic};

use crate::*;

// pub struct QueryGPUHookCx<'a> {
//   pub memory: &'a mut FunctionMemory,
//   pub dyn_cx: &'a mut DynCx,
//   pub gpu: &'a GPU,
//   pub stage: QueryHookStage<'a>,
// }

// pub enum QueryHookStage<'a> {
//   Init { cx: &'a mut ReactiveQueryCtx },
//   Unit { cx: &'a mut ReactiveQueryCtx },
//   Render,
//   Nothing,
// }

pub trait QueryGPUHookCx {
  fn use_state<T: Default>(&mut self) -> (&mut Self, &mut T);
  fn use_gpu_init<T>(&mut self, init: impl FnOnce(&GPU) -> T) -> (&mut Self, &mut T);
  fn use_multi_updater<T>(
    &mut self,
    f: impl FnOnce() -> MultiUpdateContainer<T>,
  ) -> Option<LockReadGuardHolder<MultiUpdateContainer<T>>>;
  fn use_multi_updater_gpu<T>(
    &mut self,
    f: impl FnOnce(&GPU) -> MultiUpdateContainer<T>,
  ) -> Option<LockReadGuardHolder<MultiUpdateContainer<T>>>;

  fn use_gpu_general_query<T: ReactiveGeneralQuery>(
    &mut self,
    f: impl FnOnce(&GPU) -> T,
  ) -> Option<T::Output>;

  fn use_uniform_buffers<K, V: Std140>(
    &mut self,
    source: impl FnOnce(UniformUpdateContainer<K, V>, &GPU) -> UniformUpdateContainer<K, V>,
  ) -> Option<LockReadGuardHolder<UniformUpdateContainer<K, V>>>;

  fn use_storage_buffer<V: Std430>(
    &mut self,
    source: impl FnOnce(&GPU) -> ReactiveStorageBufferContainer<V>,
  ) -> Option<StorageBufferReadonlyDataView<[V]>>;

  fn use_multi_updater_ref<T>(
    &mut self,
    f: impl FnOnce(&GPU) -> MultiUpdateContainer<T>,
  ) -> (&mut Self, Option<&T>);

  fn use_uniform_buffers_ref<K, V: Std140>(
    &mut self,
    source: impl FnOnce(UniformUpdateContainer<K, V>, &GPU) -> UniformUpdateContainer<K, V>,
  ) -> (&mut Self, Option<&FastHashMap<K, UniformBufferDataView<V>>>);

  fn use_global_multi_reactive_query<D: ForeignKeySemantic>(
    &mut self,
  ) -> Option<
    Box<dyn DynMultiQuery<Key = EntityHandle<D::ForeignEntity>, Value = EntityHandle<D::Entity>>>,
  >;

  fn use_reactive_query<K, V, Q: ReactiveQuery<Key = K, Value = V>>(
    &mut self,
    source: impl FnOnce() -> Q,
  ) -> Option<Box<dyn DynQuery<Key = K, Value = V>>>;

  fn use_val_refed_reactive_query<K, V, Q: ReactiveValueRefQuery<Key = K, Value = V>>(
    &mut self,
    source: impl FnOnce(&GPU) -> Q,
  ) -> Option<Box<dyn DynValueRefQuery<Key = K, Value = V>>>;

  // pub fn when_render<X>(&self, f: impl FnOnce() -> X) -> Option<X> {
  //   if let QueryHookStage::Render = self.stage {
  //     Some(f())
  //   } else {
  //     None
  //   }
  // }
  fn when_render<X>(&self, f: impl FnOnce() -> X) -> Option<X>;
  fn when_init<X>(&self, f: impl FnOnce() -> X) -> Option<X>;
}
