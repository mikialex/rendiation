use ::hook::*;
use database::*;

use crate::*;

pub struct QueryGPUHookFeatureCx<'a> {
  pub gpu: &'a GPU,
  pub query_cx: &'a mut ReactiveQueryCtx,
  pub db_linear_changes: &'a mut DBLinearChangeWatchGroup,
  pub db_query_changes: &'a mut DBQueryChangeWatchGroup,
}

pub struct QueryGPUHookCx<'a> {
  pub memory: &'a mut FunctionMemory,
  pub gpu: &'a GPU,
  pub query_cx: &'a mut ReactiveQueryCtx,
  pub db_linear_changes: &'a mut DBLinearChangeWatchGroup,
  pub db_query_changes: &'a mut DBQueryChangeWatchGroup,
  pub db_shared_rev_ref: &'a mut DBForeignKeySharedRevRefs,
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

pub enum UseResult<T> {
  SpawnStageFuture(Box<dyn Future<Output = T> + Unpin + Send>),
  SpawnStageReady(T),
  ResolveStageReady(T),
  NotInStage,
}

impl<T: 'static> UseResult<T> {
  pub fn map<U>(self, f: impl FnOnce(T) -> U + Send + 'static) -> UseResult<U> {
    use futures::FutureExt;
    match self {
      UseResult::SpawnStageFuture(fut) => UseResult::SpawnStageFuture(Box::new(fut.map(f))),
      UseResult::SpawnStageReady(t) => UseResult::SpawnStageReady(f(t)),
      UseResult::ResolveStageReady(t) => UseResult::ResolveStageReady(f(t)),
      UseResult::NotInStage => UseResult::NotInStage,
    }
  }

  pub fn expect_resolve_stage(self) -> T {
    match self {
      UseResult::ResolveStageReady(t) => t,
      _ => panic!("expect spawn stage ready"),
    }
  }

  pub fn expect_spawn_stage_ready(self) -> T {
    match self {
      UseResult::SpawnStageReady(t) => t,
      _ => panic!("expect spawn stage ready"),
    }
  }

  pub fn filter_map_changes<X, U>(
    self,
    f: impl Fn(X) -> Option<U> + Clone + Sync + Send + 'static,
  ) -> UseResult<impl DataChanges<Key = T::Key, Value = U>>
  where
    T: DataChanges<Value = X>,
    U: CValue,
  {
    self.map(|t| t.collective_filter_map(f))
  }

  pub fn map_changes<X, U>(
    self,
    f: impl Fn(X) -> U + Clone + Sync + Send + 'static,
  ) -> UseResult<impl DataChanges<Key = T::Key, Value = U>>
  where
    T: DataChanges<Value = X>,
    U: CValue,
  {
    self.map(|t| t.collective_map(f))
  }
}

pub enum QueryStageResult<T, U> {
  Update(T),
  Result(U),
  Others,
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
  ) -> UseResult<Arc<LinearBatchChanges<u32, C::Data>>> {
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
      UseResult::SpawnStageReady(cx.db_linear_changes.get_buffered_changes::<C>(tk.0))
    } else {
      UseResult::NotInStage
    }
  }

  pub fn use_dual_query<C: ComponentSemantic>(&mut self) -> UseResult<DBDualQuery<C::Data>> {
    self.use_query_change::<C>().map(|change| DualQuery {
      view: get_db_view::<C>(),
      delta: change,
    })
  }

  pub fn use_query_change<C: ComponentSemantic>(&mut self) -> UseResult<DBChange<C::Data>> {
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
      UseResult::SpawnStageReady(cx.db_query_changes.get_buffered_changes::<C>(tk.0))
    } else {
      UseResult::NotInStage
    }
  }

  #[track_caller]
  pub fn use_db_rev_ref<C: ForeignKeySemantic>(
    &mut self,
  ) -> QueryStageResult<impl Future<Output = RevRefForeignKey>, RevRefForeignKey> {
    if let Some(task_id) = self
      .db_shared_rev_ref
      .task_id_mapping
      .get(&C::component_id())
    {
      return match &self.stage {
        GPUQueryHookStage::Update { task_pool, .. } => {
          QueryStageResult::Update(task_pool.share_task_by_id(*task_id))
        }
        GPUQueryHookStage::CreateRender { task, .. } => {
          QueryStageResult::Result(task.expect_result_by_id(*task_id))
        }
      };
    } else {
      self.scope(|cx| {
        let changes = cx.use_query_change::<C>();
        let result = cx.use_rev_ref(changes);
        if let TaskUseResult::SpawnId(task_id) = result {
          cx.db_shared_rev_ref
            .task_id_mapping
            .insert(C::component_id(), task_id);
        }
      })
    }
    self.use_db_rev_ref::<C>()
  }

  pub fn use_rev_ref<V: CKey, C: Query<Value = ValueChange<V>> + 'static>(
    &mut self,
    changes: UseResult<C>,
  ) -> TaskUseResult<RevRefContainerRead<V, C::Key>> {
    let (_, mapping) = self.use_plain_state_default_cloned::<RevRefContainer<V, C::Key>>();
    self.use_task_result_by_fn(move || {
      bookkeeping_hash_relation(&mut mapping.write(), changes.expect_spawn_stage_ready());
      mapping.make_read_holder()
    })
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

pub trait ForeignKeyLikeChangesExt: DataChanges<Value = Option<RawEntityHandle>> {
  fn map_some_u32_index(self) -> impl DataChanges<Key = Self::Key, Value = u32> {
    self.collective_filter_map(|id| id.map(|v| v.index()))
  }
  fn map_u32_index_or_u32_max(self) -> impl DataChanges<Key = Self::Key, Value = u32> {
    self.collective_map(|id| id.map(|v| v.index()).unwrap_or(u32::MAX))
  }
}
impl<T: DataChanges<Value = Option<RawEntityHandle>>> ForeignKeyLikeChangesExt for T {}
