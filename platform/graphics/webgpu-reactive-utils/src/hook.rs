use ::hook::*;
use database::*;

use crate::*;

pub struct QueryGPUHookFeatureCx<'a> {
  pub gpu: &'a GPU,
  pub query_cx: &'a mut ReactiveQueryCtx,
  pub db_watch_scope: &'a mut DBWatchScope,
}

pub struct QueryGPUHookCx<'a> {
  pub memory: &'a mut FunctionMemory,
  pub gpu: &'a GPU,
  pub query_cx: &'a mut ReactiveQueryCtx,
  pub db_watch_scope: &'a mut DBWatchScope,
  pub shared_results: &'a mut SharedHookResult,
  pub stage: GPUQueryHookStage<'a>,
}

#[non_exhaustive]
pub enum GPUQueryHookStage<'a> {
  Update {
    task_pool: &'a mut AsyncTaskPool,
    spawner: &'a TaskSpawner,
  },
  CreateRender {
    query: QueryResultCtx,
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
      query_cx: self.query_cx,
      db_watch_scope: self.db_watch_scope,
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
          query_cx: self.query_cx,
          db_watch_scope: self.db_watch_scope,
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

  pub fn use_begin_change_set_collect(
    &mut self,
  ) -> (&mut Self, impl FnOnce(&mut Self) -> Option<bool>) {
    let (qcx, set) = self.use_state_init(QueryCtxSetInfo::default);

    // as the dynamic scope can be nested in the scope, we need maintain the set
    // per call cycle to make sure the watch set is up to date
    qcx.query_cx.record_new_registered(set);

    // todo, how to avoid this?
    let set: &mut QueryCtxSetInfo = unsafe { std::mem::transmute(set) };

    (self, |qcx: &mut Self| {
      qcx.query_cx.end_record(set);
      if let GPUQueryHookStage::CreateRender { query, .. } = &qcx.stage {
        query.has_any_changed_in_set(set).into()
      } else {
        None
      }
    })
  }

  pub fn use_multi_updater_gpu<T: 'static>(
    &mut self,
    f: impl FnOnce(&GPU) -> MultiUpdateContainer<T>,
  ) -> Option<LockReadGuardHolder<MultiUpdateContainer<T>>> {
    let (cx, token) =
      self.use_state_with_features(|cx| cx.query_cx.register_multi_updater(f(cx.gpu)));

    if let GPUQueryHookStage::CreateRender { query, .. } = &mut cx.stage {
      query.take_multi_updater_updated::<T>(*token)
    } else {
      None
    }
  }

  pub fn use_multi_updater<T: 'static>(
    &mut self,
    f: impl FnOnce() -> MultiUpdateContainer<T>,
  ) -> Option<LockReadGuardHolder<MultiUpdateContainer<T>>> {
    self.use_multi_updater_gpu(|_| f())
  }

  pub fn use_gpu_general_query<T: ReactiveGeneralQuery + 'static>(
    &mut self,
    f: impl FnOnce(&GPU) -> T,
  ) -> Option<T::Output> {
    let (cx, token) = self.use_state_with_features(|cx| cx.query_cx.register_typed(f(cx.gpu)));

    if let GPUQueryHookStage::CreateRender { query, .. } = &mut cx.stage {
      Some(
        *query
          .take_result(*token)
          .unwrap()
          .downcast::<T::Output>()
          .unwrap(),
      )
    } else {
      None
    }
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
    let (_, uniform) = self.use_plain_state_default_cloned::<UniformBufferCollection<K, V>>();
    uniform
  }

  pub fn use_uniform_array_buffers<V: Std140, const N: usize>(
    &mut self,
    f: impl FnOnce(&GPU) -> UniformArrayUpdateContainer<V, N>,
  ) -> Option<UniformBufferDataView<Shader140Array<V, N>>> {
    let (cx, token) =
      self.use_state_with_features(|cx| cx.query_cx.register_multi_updater(f(cx.gpu)));

    if let GPUQueryHookStage::CreateRender { query, .. } = &mut cx.stage {
      query.take_uniform_array_buffer(*token)
    } else {
      None
    }
  }

  pub fn use_uniform_array_buffers2<V: Std140 + Default, const N: usize>(
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

  pub fn use_storage_buffer<V: Std430>(
    &mut self,
    f: impl FnOnce(&GPU) -> ReactiveStorageBufferContainer<V>,
  ) -> Option<StorageBufferReadonlyDataView<[V]>> {
    let (cx, token) =
      self.use_state_with_features(|cx| cx.query_cx.register_multi_updater(f(cx.gpu)));

    if let GPUQueryHookStage::CreateRender { query, .. } = &mut cx.stage {
      query.take_storage_array_buffer(*token)
    } else {
      None
    }
  }

  pub fn use_reactive_query_gpu<K, V, Q>(
    &mut self,
    f: impl FnOnce(&GPU) -> Q,
  ) -> Option<Box<dyn DynQuery<Key = K, Value = V>>>
  where
    K: CKey,
    V: CValue,
    Q: ReactiveQuery<Key = K, Value = V> + Unpin,
  {
    let (cx, token) =
      self.use_state_with_features(|cx| cx.query_cx.register_reactive_query(f(cx.gpu)));

    if let GPUQueryHookStage::CreateRender { query, .. } = &mut cx.stage {
      query.take_reactive_query_updated(*token)
    } else {
      None
    }
  }

  pub fn use_val_refed_reactive_query<K, V, Q>(
    &mut self,
    f: impl FnOnce(&GPU) -> Q,
  ) -> Option<Box<dyn DynValueRefQuery<Key = K, Value = V>>>
  where
    K: CKey,
    V: CValue,
    Q: ReactiveValueRefQuery<Key = K, Value = V>,
  {
    let (cx, token) =
      self.use_state_with_features(|cx| cx.query_cx.register_val_refed_reactive_query(f(cx.gpu)));

    if let GPUQueryHookStage::CreateRender { query, .. } = &mut cx.stage {
      query.take_val_refed_reactive_query_updated(*token)
    } else {
      None
    }
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

struct NothingToDrop<T>(T);

impl<T> CanCleanUpFrom<QueryGPUHookDropCx<'_>> for NothingToDrop<T> {
  fn drop_from_cx(&mut self, _: &mut QueryGPUHookDropCx) {}
}

pub struct QueryGPUHookDropCx<'a> {
  pub query_cx: &'a mut ReactiveQueryCtx,
  pub db_watch_scope: &'a mut DBWatchScope,
}

impl QueryHookCxLike for QueryGPUHookCx<'_> {
  fn shared_ctx(&mut self) -> &mut SharedHookResult {
    self.shared_results
  }

  fn is_spawning_stage(&self) -> bool {
    matches!(&self.stage, GPUQueryHookStage::Update { .. })
  }
  fn stage(&mut self) -> QueryHookStage {
    match &mut self.stage {
      GPUQueryHookStage::Update {
        spawner, task_pool, ..
      } => QueryHookStage::SpawnTask {
        spawner,
        pool: task_pool,
      },
      GPUQueryHookStage::CreateRender { task, .. } => QueryHookStage::ResolveTask { task },
    }
  }
}

impl DBHookCxLike for QueryGPUHookCx<'_> {
  fn use_changes<C: ComponentSemantic>(
    &mut self,
  ) -> UseResult<Arc<LinearBatchChanges<u32, C::Data>>> {
    struct WatchToken(u32, ComponentId);
    impl CanCleanUpFrom<QueryGPUHookDropCx<'_>> for WatchToken {
      fn drop_from_cx(&mut self, cx: &mut QueryGPUHookDropCx<'_>) {
        cx.db_watch_scope
          .change
          .notify_consumer_dropped(self.1, self.0);
      }
    }

    let (cx, tk) = self.use_state_with_features(|cx| {
      let id = cx.db_watch_scope.change.allocate_next_consumer_id();
      WatchToken(id, C::component_id())
    });

    if let GPUQueryHookStage::Update { .. } = &cx.stage {
      UseResult::SpawnStageReady(cx.db_watch_scope.change.get_buffered_changes::<C>(tk.0))
    } else {
      UseResult::NotInStage
    }
  }

  fn use_query_set<E: EntitySemantic>(&mut self) -> UseResult<DBDelta<()>> {
    struct WatchToken(u32, EntityId);
    impl CanCleanUpFrom<QueryGPUHookDropCx<'_>> for WatchToken {
      fn drop_from_cx(&mut self, cx: &mut QueryGPUHookDropCx<'_>) {
        cx.db_watch_scope
          .query_set
          .notify_consumer_dropped(self.1, self.0);
      }
    }

    let (cx, tk) = self.use_state_with_features(|cx| {
      let id = cx.db_watch_scope.query_set.allocate_next_consumer_id();
      WatchToken(id, E::entity_id())
    });

    if let GPUQueryHookStage::Update { .. } = &cx.stage {
      UseResult::SpawnStageReady(cx.db_watch_scope.query_set.get_buffered_changes::<E>(tk.0))
    } else {
      UseResult::NotInStage
    }
  }

  fn use_query_change<C: ComponentSemantic>(&mut self) -> UseResult<DBDelta<C::Data>> {
    struct WatchToken(u32, ComponentId);
    impl CanCleanUpFrom<QueryGPUHookDropCx<'_>> for WatchToken {
      fn drop_from_cx(&mut self, cx: &mut QueryGPUHookDropCx<'_>) {
        cx.db_watch_scope
          .query
          .notify_consumer_dropped(self.1, self.0);
      }
    }

    let (cx, tk) = self.use_state_with_features(|cx| {
      let id = cx.db_watch_scope.query.allocate_next_consumer_id();
      WatchToken(id, C::component_id())
    });

    if let GPUQueryHookStage::Update { .. } = &cx.stage {
      UseResult::SpawnStageReady(cx.db_watch_scope.query.get_buffered_changes::<C>(tk.0))
    } else {
      UseResult::NotInStage
    }
  }
}
