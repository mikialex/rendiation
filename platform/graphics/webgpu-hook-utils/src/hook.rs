use ::hook::*;

use crate::*;

pub struct QueryGPUHookFeatureCx<'a> {
  pub gpu: &'a GPU,
  pub shared_ctx: &'a mut SharedHooksCtx,
  pub storage_allocator: &'a dyn AbstractStorageAllocator,
}

pub struct QueryGPUHookCx<'a> {
  pub memory: &'a mut FunctionMemory,
  pub gpu: &'a GPU,
  pub storage_allocator: Box<dyn AbstractStorageAllocator>,
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
  Inspect(&'a mut dyn Inspector),
}

pub trait Inspector {
  fn label(&mut self, label: &str);
}

unsafe impl<'a> HooksCxLike for QueryGPUHookCx<'a> {
  fn memory_mut(&mut self) -> &mut FunctionMemory {
    self.memory
  }

  fn memory_ref(&self) -> &FunctionMemory {
    self.memory
  }

  fn is_dynamic_stage(&self) -> bool {
    matches!(&self.stage, GPUQueryHookStage::Update { .. })
  }

  fn flush(&mut self) {
    if let GPUQueryHookStage::Update { .. } = self.stage {
      let mut drop_cx = QueryGPUHookDropCx {
        share_cx: self.shared_ctx,
      };
      let drop_cx = &mut drop_cx as *mut _ as *mut ();
      self.memory.flush(drop_cx);
    }
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
          storage_allocator: &self.storage_allocator,
        })
      },
      |state: &mut T, dcx: &mut ()| {
        let dcx: &mut QueryGPUHookDropCx = unsafe { std::mem::transmute(dcx) };
        T::drop_from_cx(state, dcx);
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

  pub fn use_gpu_init<T: 'static>(
    &mut self,
    init: impl FnOnce(&GPU, &dyn AbstractStorageAllocator) -> T,
  ) -> (&mut Self, &mut T) {
    let (cx, state) =
      self.use_state_with_features(|cx| NothingToDrop(init(cx.gpu, cx.storage_allocator)));
    (cx, &mut state.0)
  }

  pub fn use_gpu_multi_access_states(
    &mut self,
    init: MultiAccessGPUDataBuilderInit,
  ) -> (&mut Self, &mut MultiAccessGPUStates) {
    self.use_gpu_init(|gpu, alloc| MultiAccessGPUStates::new(gpu, init, alloc))
  }

  pub fn use_uniform_buffers<K: 'static, V: Std140 + 'static>(
    &mut self,
  ) -> UniformBufferCollection<K, V> {
    self.use_shared_hash_map()
  }

  pub fn use_uniform_array_buffers<V: Std140 + Default, const N: usize>(
    &mut self,
  ) -> (&mut Self, &mut UniformBufferDataView<Shader140Array<V, N>>) {
    self.use_gpu_init(|gpu, _| UniformBufferDataView::create_default(&gpu.device))
  }

  pub fn use_storage_buffer<V: Std430 + ShaderSizedValueNodeType>(
    &mut self,
    label: &str,
    init_capacity_item_count: u32,
    max_item_count: u32,
  ) -> (&mut Self, &mut SparseUpdateStorageBuffer<V>) {
    let (cx, storage) = self.use_gpu_init(|gpu, alloc| {
      SparseUpdateStorageBuffer::new(label, init_capacity_item_count, max_item_count, alloc, gpu)
    });

    if let GPUQueryHookStage::Update { .. } = &mut cx.stage {
      storage.collector = Some(Default::default());
    }

    if let GPUQueryHookStage::Inspect(inspector) = &mut cx.stage {
      let buffer_size: u64 = storage.get_gpu_buffer().byte_size();
      let buffer_size = buffer_size as f32 / 1024.;
      inspector.label(&format!("storage: {}, size: {:.2} kb", label, buffer_size));
    }

    (cx, storage)
  }

  pub fn when_render<X>(&self, f: impl FnOnce() -> X) -> Option<X> {
    self.is_in_render().then(f)
  }
  pub fn is_in_render(&self) -> bool {
    matches!(&self.stage, GPUQueryHookStage::CreateRender { .. })
  }
}

impl<T> CanCleanUpFrom<QueryGPUHookDropCx<'_>> for NothingToDrop<T> {
  fn drop_from_cx(&mut self, _: &mut QueryGPUHookDropCx) {}
}

pub struct QueryGPUHookDropCx<'a> {
  pub share_cx: &'a mut SharedHooksCtx,
}

impl CanCleanUpFrom<QueryGPUHookDropCx<'_>> for SharedConsumerToken {
  fn drop_from_cx(&mut self, cx: &mut QueryGPUHookDropCx<'_>) {
    if let Some(mem) = cx.share_cx.drop_consumer(*self) {
      mem.write().memory.cleanup_assume_only_plain_states();
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
      SharedConsumerToken(id, key)
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
      _ => QueryHookStage::Other,
    }
  }
}

impl database::DBHookCxLike for QueryGPUHookCx<'_> {}
