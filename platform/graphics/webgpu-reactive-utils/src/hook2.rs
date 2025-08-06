use ::hook::*;
use database::*;

use crate::*;

pub struct GPUResourceCx<'a> {
  memory: &'a mut FunctionMemory,
  pub async_cx: &'a mut AsyncQueryCtx,
}

unsafe impl<'a> HooksCxLike for GPUResourceCx<'a> {
  fn memory_mut(&mut self) -> &mut FunctionMemory {
    todo!()
  }

  fn memory_ref(&self) -> &FunctionMemory {
    todo!()
  }

  fn flush(&mut self) {
    todo!()
  }
}

// note, as we allow the use_xx in used every where without limits, we should using call_location to remap states
impl<'a> GPUResourceCx<'a> {
  pub fn use_changes<C: ComponentSemantic>(&mut self) -> Arc<LinearBatchChanges<C::Data>> {
    todo!()
  }

  pub fn use_query_compute<C: ComponentSemantic>(
    &mut self,
  ) -> impl AsyncQueryCompute<Key = EntityHandle<C::Entity>, Value = C::Data> {
    (EmptyQuery::default(), EmptyQuery::default())
  }

  pub fn use_uniform_buffers<K, T: Std140>(&mut self) -> UniformBufferCollection<K, T> {
    todo!()
  }
}

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
