use ::hook::*;

use crate::*;

pub struct QueryGPUHookFeatureCx<'a> {
  pub gpu: &'a GPU,
  pub shared_ctx: &'a mut SharedHooksCtx,
}

pub struct QueryGPUHookCx<'a> {
  pub memory: &'a mut FunctionMemory,
  pub gpu: &'a GPU,
  pub shared_ctx: &'a mut SharedHooksCtx,
  pub stage: GPUQueryHookStage<'a>,
}

#[non_exhaustive]
pub enum GPUQueryHookStage<'a> {
  Update {
    task_pool: &'a mut AsyncTaskPool,
    spawner: &'a TaskSpawner,
    change_collector: &'a mut ChangeCollector,
  },
  CreateRender {
    task: TaskPoolResultCx,
  },
}

unsafe impl<'a> HooksCxLike for QueryGPUHookCx<'a> {
  fn memory_mut(&mut self) -> &mut FunctionMemory {
    self.memory
  }

  fn memory_ref(&self) -> &FunctionMemory {
    self.memory
  }

  fn flush(&mut self) {
    let mut drop_cx = QueryGPUHookDropCx {
      share_cx: self.shared_ctx,
    };
    self.memory.flush(&mut drop_cx as *mut _ as *mut ());
  }

  fn use_plain_state<T: 'static>(&mut self, f: impl FnOnce() -> T) -> (&mut Self, &mut T) {
    let (cx, state) = self.use_state_init(|| NothingToDrop(f()));
    (cx, &mut state.0)
  }
}

impl<'a> QueryGPUHookCx<'a> {
  pub fn use_state_with_features<T: 'static + for<'x> CanCleanUpFrom<QueryGPUHookDropCx<'x>>>(
    &mut self,
    init: impl FnOnce(QueryGPUHookFeatureCx) -> T,
  ) -> (&mut Self, &mut T) {
    let s = unsafe { std::mem::transmute_copy(&self) };

    let state = self.memory.expect_state_init(
      || {
        init(QueryGPUHookFeatureCx {
          gpu: self.gpu,
          shared_ctx: self.shared_ctx,
        })
      },
      |state: &mut T, dcx: &mut ()| {
        let dcx: &mut QueryGPUHookDropCx = unsafe { std::mem::transmute(dcx) };
        T::drop_from_cx(state, dcx);
        unsafe { core::ptr::drop_in_place(state) }
      },
    );

    (s, state)
  }

  pub fn use_state<T: Default + for<'x> CanCleanUpFrom<QueryGPUHookDropCx<'x>> + 'static>(
    &mut self,
  ) -> (&mut Self, &mut T) {
    self.use_state_init(T::default)
  }

  pub fn use_state_init<T: 'static + for<'x> CanCleanUpFrom<QueryGPUHookDropCx<'x>>>(
    &mut self,
    init: impl FnOnce() -> T,
  ) -> (&mut Self, &mut T) {
    let (cx, state) = self.use_state_with_features(|_| init());
    (cx, state)
  }

  pub fn use_gpu_init<T: 'static>(&mut self, init: impl FnOnce(&GPU) -> T) -> (&mut Self, &mut T) {
    let (cx, state) = self.use_state_with_features(|cx| NothingToDrop(init(cx.gpu)));
    (cx, &mut state.0)
  }

  pub fn use_gpu_multi_access_states(
    &mut self,
    init: MultiAccessGPUDataBuilderInit,
  ) -> (&mut Self, &mut MultiAccessGPUStates) {
    self.use_gpu_init(|gpu| MultiAccessGPUStates::new(gpu, init))
  }

  pub fn use_uniform_buffers<K: 'static, V: Std140 + 'static>(
    &mut self,
  ) -> UniformBufferCollection<K, V> {
    self.use_shared_hash_map()
  }

  pub fn use_uniform_array_buffers<V: Std140 + Default, const N: usize>(
    &mut self,
  ) -> (&mut Self, &mut UniformBufferDataView<Shader140Array<V, N>>) {
    self.use_gpu_init(|gpu| UniformBufferDataView::create_default(&gpu.device))
  }

  pub fn use_storage_buffer2<V: Std430>(
    &mut self,
    init_capacity_item_count: u32,
    max_item_count: u32,
  ) -> (&mut Self, &mut CommonStorageBufferImpl<V>) {
    self.use_gpu_init(|gpu| {
      create_common_storage_buffer_container(init_capacity_item_count, max_item_count, gpu)
    })
  }

  pub fn when_render<X>(&self, f: impl FnOnce() -> X) -> Option<X> {
    self.is_in_render().then(f)
  }
  pub fn is_in_render(&self) -> bool {
    matches!(&self.stage, GPUQueryHookStage::CreateRender { .. })
  }
  pub fn when_init<X>(&self, f: impl FnOnce() -> X) -> Option<X> {
    self.is_creating().then(f)
  }
}

impl<T> CanCleanUpFrom<QueryGPUHookDropCx<'_>> for NothingToDrop<T> {
  fn drop_from_cx(&mut self, _: &mut QueryGPUHookDropCx) {}
}

pub struct QueryGPUHookDropCx<'a> {
  pub share_cx: &'a mut SharedHooksCtx,
}

pub struct ShaderConsumerToken(pub u32, pub ShareKey);
impl CanCleanUpFrom<QueryGPUHookDropCx<'_>> for ShaderConsumerToken {
  fn drop_from_cx(&mut self, cx: &mut QueryGPUHookDropCx<'_>) {
    if let Some(mem) = cx.share_cx.drop_consumer(self.1, self.0) {
      mem.write().memory.cleanup(cx as *mut _ as *mut ());
    }
    // this check is necessary because not all consumer need reconcile change
    if let Some(reconciler) = cx.share_cx.reconciler.get_mut(&self.1) {
      if reconciler.remove_consumer(self.0) {
        cx.share_cx.reconciler.remove(&self.1);
      }
    }
  }
}

impl QueryHookCxLike for QueryGPUHookCx<'_> {
  fn shared_hook_ctx(&mut self) -> &mut SharedHooksCtx {
    self.shared_ctx
  }

  fn use_shared_consumer(&mut self, key: ShareKey) -> u32 {
    let (_, tk) = self.use_state_with_features(|fcx| {
      let id = fcx.shared_ctx.next_consumer_id();
      ShaderConsumerToken(id, key)
    });

    tk.0
  }

  fn is_spawning_stage(&self) -> bool {
    matches!(&self.stage, GPUQueryHookStage::Update { .. })
  }
  fn is_resolve_stage(&self) -> bool {
    matches!(&self.stage, GPUQueryHookStage::CreateRender { .. })
  }
  fn stage(&mut self) -> QueryHookStage {
    match &mut self.stage {
      GPUQueryHookStage::Update {
        spawner,
        task_pool,
        change_collector,
        ..
      } => QueryHookStage::SpawnTask {
        spawner,
        pool: task_pool,
        change_collector,
      },
      GPUQueryHookStage::CreateRender { task, .. } => QueryHookStage::ResolveTask { task },
    }
  }
}

impl database::DBHookCxLike for QueryGPUHookCx<'_> {}
