use std::{any::Any, sync::Arc, task::Waker};

use fast_hash_collection::FastHashMap;

use crate::*;

pub struct ViewerAPICx<'a> {
  pub memory: &'a mut FunctionMemory,
  pub dyn_cx: &'a mut DynCx,
  pub stage: ViewerAPICxStage<'a>,
  pub shared_ctx: &'a mut SharedHooksCtx,
  pub waker: Waker,
}

pub struct ViewerAPICxDropCx;

pub struct ViewerAPIInitCx<'a> {
  pub shared_ctx: &'a mut SharedHooksCtx,
}

impl<'a> ViewerAPICx<'a> {
  pub fn use_state_init<T>(
    &mut self,
    init: impl FnOnce(&mut ViewerAPIInitCx) -> T,
  ) -> (&mut Self, &mut T)
  where
    T: Any + CanCleanUpFrom<ViewerAPICxDropCx>,
  {
    // this is safe because user can not access previous retrieved state through returned self.
    let s = unsafe { std::mem::transmute_copy(&self) };

    let state = self.memory.expect_state_init(
      || {
        init(&mut ViewerAPIInitCx {
          shared_ctx: &mut self.shared_ctx,
        })
      },
      |state: &mut T, dcx: &mut ViewerAPICxDropCx| {
        state.drop_from_cx(dcx);
      },
    );

    (s, state)
  }
}

impl CanCleanUpFrom<ViewerAPICxDropCx> for SharedConsumerToken {
  fn drop_from_cx(&mut self, _: &mut ViewerAPICxDropCx) {}
}
impl<T> CanCleanUpFrom<ViewerAPICxDropCx> for NothingToDrop<T> {
  fn drop_from_cx(&mut self, _: &mut ViewerAPICxDropCx) {}
}

pub enum ViewerAPICxStage<'a> {
  Spawn {
    spawner: &'a TaskSpawner,
    pool: &'a mut AsyncTaskPool,
    immediate_results: &'a mut FastHashMap<u32, Arc<dyn Any + Send + Sync>>,
    change_collector: &'a mut ChangeCollector,
  },
  Resolve {
    result: &'a mut TaskPoolResultCx,
  },
}

unsafe impl<'a> HooksCxLike for ViewerAPICx<'a> {
  fn memory_mut(&mut self) -> &mut FunctionMemory {
    &mut self.memory
  }

  fn memory_ref(&self) -> &FunctionMemory {
    &self.memory
  }

  fn flush(&mut self) {
    if let ViewerAPICxStage::Spawn { .. } = &mut self.stage {
      self.memory.flush(&mut () as *mut ());
    }
  }

  fn is_dynamic_stage(&self) -> bool {
    matches!(self.stage, ViewerAPICxStage::Spawn { .. })
  }

  fn use_plain_state<T: 'static>(&mut self, f: impl FnOnce() -> T) -> (&mut Self, &mut T) {
    let (cx, s) = self.use_state_init(|_| NothingToDrop(f()));
    (cx, &mut s.0)
  }
}
impl<'a> InspectableCx for ViewerAPICx<'a> {
  fn if_inspect(&mut self, _f: impl FnOnce(&mut dyn Inspector)) {
    // do nothing
  }
}
impl<'a> QueryHookCxLike for ViewerAPICx<'a> {
  fn is_spawning_stage(&self) -> bool {
    matches!(self.stage, ViewerAPICxStage::Spawn { .. })
  }

  fn is_resolve_stage(&self) -> bool {
    matches!(self.stage, ViewerAPICxStage::Resolve { .. })
  }

  fn dyn_env(&mut self) -> &mut DynCx {
    self.dyn_cx
  }

  fn stage(&mut self) -> QueryHookStage<'_> {
    match &mut self.stage {
      ViewerAPICxStage::Spawn {
        spawner,
        pool,
        immediate_results,
        change_collector,
      } => QueryHookStage::SpawnTask {
        spawner,
        pool,
        immediate_results,
        change_collector,
      },
      ViewerAPICxStage::Resolve { result } => QueryHookStage::ResolveTask { task: result },
    }
  }

  fn waker(&mut self) -> &mut std::task::Waker {
    &mut self.waker
  }

  fn use_shared_consumer(&mut self, key: ShareKey) -> u32 {
    let (_, tk) = self.use_state_init(|fcx| {
      let id = fcx.shared_ctx.next_consumer_id();
      SharedConsumerToken(id, key)
    });

    tk.0
  }

  fn shared_hook_ctx(&mut self) -> &mut SharedHooksCtx {
    &mut self.shared_ctx
  }
}
impl<'a> DBHookCxLike for ViewerAPICx<'a> {}
