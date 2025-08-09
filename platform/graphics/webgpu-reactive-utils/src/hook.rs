use ::hook::*;
use database::*;

use crate::*;

pub struct QueryGPUHookFeatureCx<'a> {
  pub gpu: &'a GPU,
  pub query_cx: &'a mut ReactiveQueryCtx,
  pub task_pool: &'a mut AsyncTaskPool,
  pub db_linear_changes: &'a mut DBLinearChangeWatchGroup,
  pub db_query_changes: &'a mut DBQueryChangeWatchGroup,
}

pub struct QueryGPUHookCx<'a> {
  pub memory: &'a mut FunctionMemory,
  pub gpu: &'a GPU,
  pub query_cx: &'a mut ReactiveQueryCtx,
  pub db_linear_changes: &'a mut DBLinearChangeWatchGroup,
  pub db_query_changes: &'a mut DBQueryChangeWatchGroup,
  pub task_pool: &'a mut AsyncTaskPool,
  pub stage: GPUQueryHookStage<'a>,
}

pub enum GPUQueryHookStage<'a> {
  Update {
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
      db_linear_changes: self.db_linear_changes,
      db_query_changes: self.db_query_changes,
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
          db_linear_changes: self.db_linear_changes,
          db_query_changes: self.db_query_changes,
          task_pool: self.task_pool,
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

  pub fn use_changes<C: ComponentSemantic>(
    &mut self,
  ) -> Option<Arc<LinearBatchChanges<u32, C::Data>>> {
    struct WatchToken(u32, ComponentId);
    impl CanCleanUpFrom<QueryGPUHookDropCx<'_>> for WatchToken {
      fn drop_from_cx(&mut self, cx: &mut QueryGPUHookDropCx<'_>) {
        cx.db_linear_changes.notify_consumer_dropped(self.1, self.0);
      }
    }

    let (cx, tk) = self.use_state_with_features(|cx| {
      let id = cx.db_linear_changes.allocate_next_consumer_id();
      WatchToken(id, C::component_id())
    });

    if let GPUQueryHookStage::Update { .. } = &cx.stage {
      Some(cx.db_linear_changes.get_buffered_changes::<C>(tk.0))
    } else {
      None
    }
  }

  pub fn use_query_compute<C: ComponentSemantic>(&mut self) -> Option<DBComputeView<C::Data>> {
    struct WatchToken(u32, ComponentId);
    impl CanCleanUpFrom<QueryGPUHookDropCx<'_>> for WatchToken {
      fn drop_from_cx(&mut self, cx: &mut QueryGPUHookDropCx<'_>) {
        cx.db_query_changes.notify_consumer_dropped(self.1, self.0);
      }
    }

    let (cx, tk) = self.use_state_with_features(|cx| {
      let id = cx.db_query_changes.allocate_next_consumer_id();
      WatchToken(id, C::component_id())
    });

    if let GPUQueryHookStage::Update { .. } = &cx.stage {
      Some(cx.db_query_changes.get_buffered_changes::<C>(tk.0))
    } else {
      None
    }
  }

  pub fn use_uniform_buffers2<K: 'static, V: Std140 + 'static>(
    &mut self,
  ) -> UniformBufferCollection<K, V> {
    let (_, uniform) = self.use_plain_state_default_cloned::<UniformBufferCollection<K, V>>();
    uniform
  }

  pub fn use_uniform_buffers<K, V: Std140 + 'static>(
    &mut self,
    f: impl FnOnce(UniformUpdateContainer<K, V>, &GPU) -> UniformUpdateContainer<K, V>,
  ) -> Option<LockReadGuardHolder<UniformUpdateContainer<K, V>>> {
    let (cx, token) = self.use_state_with_features(|cx| {
      let source = UniformUpdateContainer::<K, V>::default();
      cx.query_cx.register_multi_updater(f(source, cx.gpu))
    });

    if let GPUQueryHookStage::CreateRender { query, .. } = &mut cx.stage {
      query.take_multi_updater_updated(*token)
    } else {
      None
    }
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

  pub fn use_storage_buffer2<V: Std430>(
    &mut self,
    init_capacity_item_count: u32,
    max_item_count: u32,
  ) -> (&mut Self, &mut CommonStorageBufferImpl<V>) {
    let gpu = self.gpu;
    self.use_plain_state(|| {
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

  pub fn use_global_multi_reactive_query<D: ForeignKeySemantic>(
    &mut self,
  ) -> Option<
    Box<dyn DynMultiQuery<Key = EntityHandle<D::ForeignEntity>, Value = EntityHandle<D::Entity>>>,
  > {
    let (cx, token) = self.use_state_with_features(|cx| {
      let query = global_rev_ref().watch_inv_ref::<D>();
      cx.query_cx.register_multi_reactive_query(query)
    });

    if let GPUQueryHookStage::CreateRender { query, .. } = &mut cx.stage {
      query.take_reactive_multi_query_updated(*token)
    } else {
      None
    }
  }

  pub fn use_reactive_query<K, V, Q>(
    &mut self,
    source: impl FnOnce() -> Q,
  ) -> Option<Box<dyn DynQuery<Key = K, Value = V>>>
  where
    K: CKey,
    V: CValue,
    Q: ReactiveQuery<Key = K, Value = V> + Unpin,
  {
    self.use_reactive_query_gpu(|_| source())
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
  pub db_linear_changes: &'a mut DBLinearChangeWatchGroup,
  pub db_query_changes: &'a mut DBQueryChangeWatchGroup,
}

impl QueryHookCxLike for QueryGPUHookCx<'_> {
  fn is_spawning_stage(&self) -> bool {
    matches!(&self.stage, GPUQueryHookStage::Update { .. })
  }
  fn stage(&mut self) -> QueryHookStage {
    match &mut self.stage {
      GPUQueryHookStage::Update { spawner } => QueryHookStage::SpawnTask { spawner },
      GPUQueryHookStage::CreateRender { task, .. } => QueryHookStage::ResolveTask { task },
    }
  }

  fn pool(&mut self) -> &mut AsyncTaskPool {
    self.task_pool
  }
}

pub trait ForeignKeyLikeChangesExt: DataChanges<Value = Option<RawEntityHandle>> {
  fn map_some_u32_index(self) -> impl DataChanges<Key = Self::Key, Value = u32> {
    self.collective_filter_map(|id| id.map(|v| v.index()))
  }
  fn map_u32_index_or_u32_max(self) -> impl DataChanges<Key = Self::Key, Value = u32> {
    self.collective_map(|id| id.map(|v| v.index()).unwrap_or(u32::MAX))
  }
}
impl<T: DataChanges<Value = Option<RawEntityHandle>>> ForeignKeyLikeChangesExt for T {}
