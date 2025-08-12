use std::{any::TypeId, marker::PhantomData};

use ::hook::*;
use database::*;
use futures::*;

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

#[derive(Default)]
pub struct SharedHookResult {
  pub task_id_mapping: FastHashMap<ShareKey, u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShareKey {
  TypeId(TypeId),
  Hash(u64),
}

impl SharedHookResult {
  pub fn reset(&mut self) {
    self.task_id_mapping.clear();
  }
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

  #[track_caller]
  pub fn use_result<T: Send + Sync + 'static + Clone>(&mut self, re: UseResult<T>) -> UseResult<T> {
    let (cx, spawned) = self.use_plain_state_default();
    if cx.is_spawning_stage() || *spawned {
      let fut = match re {
        UseResult::SpawnStageFuture(future) => {
          *spawned = true;
          Some(future)
        }
        UseResult::SpawnStageReady(re) => return UseResult::SpawnStageReady(re),
        _ => {
          if cx.is_spawning_stage() {
            panic!("must contain work in spawning stage")
          } else {
            None
          }
        }
      };

      cx.scope(|cx| match cx.use_future(fut) {
        TaskUseResult::SpawnId(_) => UseResult::NotInStage,
        TaskUseResult::Result(r) => UseResult::ResolveStageReady(r),
      })
    } else {
      re
    }
  }

  pub fn use_changes<C: ComponentSemantic>(
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

  #[track_caller]
  pub fn use_db_rev_ref_tri_view<C: ForeignKeySemantic>(
    &mut self,
  ) -> UseResult<RevRefForeignTriQuery> {
    let rev_many_view = self.use_db_rev_ref::<C>();
    let changes = self.use_query_change::<C>();
    // this generate less code compare to join i assume
    if self.is_spawning_stage() {
      let rev_many_view = rev_many_view.expect_spawn_stage_future();
      let changes = changes.expect_spawn_stage_ready();

      let changes = FilterMapQueryChange {
        base: changes,
        mapper: |v| v,
      }
      .into_boxed();

      UseResult::SpawnStageFuture(Box::new(rev_many_view.map(move |rev_many_view| {
        RevRefForeignTriQuery {
          base: DualQuery {
            view: get_db_view::<C>().filter_map(|v| v).into_boxed(),
            delta: changes,
          },
          rev_many_view,
        }
      })))
    } else {
      UseResult::NotInStage
    }
  }

  #[track_caller]
  pub fn use_db_rev_ref_typed<C: ForeignKeySemantic>(
    &mut self,
  ) -> UseResult<RevRefForeignKeyReadTyped<C>> {
    self
      .use_db_rev_ref::<C>()
      .map(|v| RevRefForeignKeyReadTyped {
        internal: v,
        phantom: PhantomData,
      })
  }

  #[track_caller]
  pub fn use_db_rev_ref<C: ForeignKeySemantic>(&mut self) -> UseResult<RevRefForeignKeyRead> {
    let key = match C::component_id() {
      ComponentId::TypeId(type_id) => ShareKey::TypeId(type_id),
      ComponentId::Hash(hash) => ShareKey::Hash(hash),
    };

    self.use_shared_compute_internal(
      |cx| {
        let changes = cx
          .use_query_change::<C>()
          .map(|v| v.delta_filter_map(|v| v));

        cx.use_rev_ref(changes)
      },
      key,
    )
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

  #[track_caller]
  pub fn use_shared_compute<
    T: Clone + Send + Sync + 'static,
    F: Fn(&mut Self) -> UseResult<T> + 'static,
  >(
    &mut self,
    logic: F,
  ) -> UseResult<T> {
    let key = ShareKey::TypeId(TypeId::of::<T>());
    self.use_shared_compute_internal(
      move |cx| {
        let result = logic(cx);
        cx.use_future(result.if_spawn_stage_future())
      },
      key,
    )
  }

  #[track_caller]
  pub fn use_shared_compute_internal<
    T: Clone + Send + Sync + 'static,
    F: Fn(&mut Self) -> TaskUseResult<T> + 'static,
  >(
    &mut self,
    logic: F,
    key: ShareKey,
  ) -> UseResult<T> {
    if let Some(task_id) = self.shared_results.task_id_mapping.get(&key) {
      return match &self.stage {
        GPUQueryHookStage::Update { task_pool, .. } => {
          UseResult::SpawnStageFuture(task_pool.share_task_by_id(*task_id))
        }
        GPUQueryHookStage::CreateRender { task, .. } => {
          UseResult::ResolveStageReady(task.expect_result_by_id(*task_id))
        }
      };
    } else {
      self.scope(|cx| {
        let result = logic(cx);
        if let TaskUseResult::SpawnId(task_id) = result {
          cx.shared_results.task_id_mapping.insert(key, task_id);
        }
      })
    }
    self.use_shared_compute_internal(logic, key)
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
  pub db_watch_scope: &'a mut DBWatchScope,
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
