use ::hook::*;

use crate::*;

pub struct QueryGPUHookCx<'a> {
  pub memory: &'a mut FunctionMemory,
  pub dyn_cx: &'a mut DynCx,
  pub gpu: &'a GPU,
  pub stage: QueryHookStage<'a>,
}

impl<'a> QueryGPUHookCx<'a> {
  pub fn use_uniform_buffers<K, V: Std140>(
    &mut self,
    source: impl FnOnce(UniformUpdateContainer<K, V>, &GPU) -> UniformUpdateContainer<K, V>,
  ) -> Option<LockReadGuardHolder<UniformUpdateContainer<K, V>>> {
    todo!()
  }

  pub fn when_create_impl<X>(&self, f: impl FnOnce() -> X) -> Option<X> {
    if let QueryHookStage::CreateImpl = self.stage {
      Some(f())
    } else {
      None
    }
  }
}

pub enum QueryHookStage<'a> {
  Init { cx: &'a mut ReactiveQueryCtx },
  Unit { cx: &'a mut ReactiveQueryCtx },
  CreateImpl,
}
