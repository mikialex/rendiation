use ::hook::*;
use database::*;

use crate::*;

pub struct QueryGPUHookFeatureCx<'a> {
  pub gpu: &'a GPU,
  pub query_cx: &'a mut ReactiveQueryCtx,
  pub task_pool: &'a mut AsyncTaskPool,
  pub linear_changes: &'a mut DBLinearChangeWatchGroup,
}

pub trait QueryGPUHookCx: HooksCxLike {
  fn gpu(&self) -> &GPU;
  fn get_stage(&self) -> &QueryHookStage;
  fn get_stage_mut(&mut self) -> &mut QueryHookStage;
  fn get_query_cx(&mut self) -> &mut ReactiveQueryCtx;
  fn get_task_pool(&mut self) -> &mut AsyncTaskPool;
  fn get_linear_changes(&mut self) -> &mut DBLinearChangeWatchGroup;
  fn get_task_spawner(&mut self) -> &TaskSpawner;

  fn use_state_with_features<T: 'static + for<'a> CanCleanUpFrom<QueryGPUHookDropCx<'a>>>(
    &mut self,
    init: impl FnOnce(QueryGPUHookFeatureCx) -> T,
  ) -> (&mut Self, &mut T);

  fn use_state<T: Default + for<'a> CanCleanUpFrom<QueryGPUHookDropCx<'a>> + 'static>(
    &mut self,
  ) -> (&mut Self, &mut T) {
    self.use_state_init(T::default)
  }

  fn use_state_init<T: 'static + for<'a> CanCleanUpFrom<QueryGPUHookDropCx<'a>>>(
    &mut self,
    init: impl FnOnce() -> T,
  ) -> (&mut Self, &mut T) {
    let (cx, state) = self.use_state_with_features(|_| init());
    (cx, state)
  }

  fn use_gpu_init<T: 'static>(&mut self, init: impl FnOnce(&GPU) -> T) -> (&mut Self, &mut T) {
    let (cx, state) = self.use_state_with_features(|cx| NothingToDrop(init(cx.gpu)));
    (cx, &mut state.0)
  }

  fn use_begin_change_set_collect(
    &mut self,
  ) -> (&mut Self, impl FnOnce(&mut Self) -> Option<bool>) {
    let (qcx, set) = self.use_state_init(QueryCtxSetInfo::default);

    // as the dynamic scope can be nested in the scope, we need maintain the set
    // per call cycle to make sure the watch set is up to date
    qcx.get_query_cx().record_new_registered(set);

    // todo, how to avoid this?
    let set: &mut QueryCtxSetInfo = unsafe { std::mem::transmute(set) };

    (self, |qcx: &mut Self| {
      qcx.get_query_cx().end_record(set);
      if let QueryHookStage::CreateRender { query, .. } = qcx.get_stage() {
        query.has_any_changed_in_set(set).into()
      } else {
        None
      }
    })
  }

  fn use_multi_updater_gpu<T: 'static>(
    &mut self,
    f: impl FnOnce(&GPU) -> MultiUpdateContainer<T>,
  ) -> Option<LockReadGuardHolder<MultiUpdateContainer<T>>> {
    let (cx, token) =
      self.use_state_with_features(|cx| cx.query_cx.register_multi_updater(f(cx.gpu)));

    if let QueryHookStage::CreateRender { query, .. } = cx.get_stage_mut() {
      query.take_multi_updater_updated::<T>(*token)
    } else {
      None
    }
  }

  fn use_multi_updater<T: 'static>(
    &mut self,
    f: impl FnOnce() -> MultiUpdateContainer<T>,
  ) -> Option<LockReadGuardHolder<MultiUpdateContainer<T>>> {
    self.use_multi_updater_gpu(|_| f())
  }

  fn use_gpu_general_query<T: ReactiveGeneralQuery + 'static>(
    &mut self,
    f: impl FnOnce(&GPU) -> T,
  ) -> Option<T::Output> {
    let (cx, token) = self.use_state_with_features(|cx| cx.query_cx.register_typed(f(cx.gpu)));

    if let QueryHookStage::CreateRender { query, .. } = cx.get_stage_mut() {
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

  fn use_changes<C: ComponentSemantic>(&mut self) -> Option<Arc<LinearBatchChanges<C::Data>>> {
    struct WatchToken(u32, ComponentId);
    impl CanCleanUpFrom<QueryGPUHookDropCx<'_>> for WatchToken {
      fn drop_from_cx(&mut self, cx: &mut QueryGPUHookDropCx<'_>) {
        cx.db_linear_changes.notify_consumer_dropped(self.1, self.0);
      }
    }

    let (cx, tk) = self.use_state_with_features(|cx| {
      let id = cx.linear_changes.allocate_next_consumer_id();
      WatchToken(id, C::component_id())
    });

    if let QueryHookStage::Update = cx.get_stage() {
      Some(cx.get_linear_changes().get_buffered_changes::<C>(tk.0))
    } else {
      None
    }
  }

  // fn use_query_compute<C: ComponentSemantic>(
  //   &mut self,
  // ) -> impl AsyncQueryCompute<Key = EntityHandle<C::Entity>, Value = C::Data> {
  //   (EmptyQuery::default(), EmptyQuery::default())
  // }

  fn use_uniform_buffers2<K: 'static, V: Std140 + 'static>(
    &mut self,
  ) -> UniformBufferCollection<K, V> {
    let (_, uniform) = self.use_state::<UniformBufferCollection<K, V>>();
    uniform.clone()
  }

  fn use_task_result<R, F>(&mut self, create_task: impl Fn(&TaskSpawner) -> F) -> Option<R>
  where
    R: 'static,
    F: Future<Output = R> + Send + 'static,
  {
    struct TaskToken(u32);
    impl CanCleanUpFrom<QueryGPUHookDropCx<'_>> for TaskToken {
      fn drop_from_cx(&mut self, _: &mut QueryGPUHookDropCx<'_>) {
        // noop
      }
    }

    let task = self.spawn_task_when_update(create_task);
    let (cx, token) = self.use_state_init(|| TaskToken(u32::MAX));

    match cx.get_stage_mut() {
      QueryHookStage::Update => {
        token.0 = cx.get_task_pool().install_task(task.unwrap());
        None
      }
      QueryHookStage::CreateRender { task, .. } => {
        let result = task
          .token_based_result
          .remove(&token.0)
          .unwrap()
          .downcast()
          .unwrap();
        Some(*result)
      }
    }
  }

  fn spawn_task_when_update<R, F: Future<Output = R>>(
    &mut self,
    create_task: impl Fn(&TaskSpawner) -> F,
  ) -> Option<F> {
    match self.get_stage_mut() {
      QueryHookStage::Update => {
        let task = create_task(self.get_task_spawner());
        Some(task)
      }
      _ => None,
    }
  }

  fn use_uniform_buffers<K, V: Std140 + 'static>(
    &mut self,
    f: impl FnOnce(UniformUpdateContainer<K, V>, &GPU) -> UniformUpdateContainer<K, V>,
  ) -> Option<LockReadGuardHolder<UniformUpdateContainer<K, V>>> {
    let (cx, token) = self.use_state_with_features(|cx| {
      let source = UniformUpdateContainer::<K, V>::default();
      cx.query_cx.register_multi_updater(f(source, cx.gpu))
    });

    if let QueryHookStage::CreateRender { query, .. } = cx.get_stage_mut() {
      query.take_multi_updater_updated(*token)
    } else {
      None
    }
  }

  fn use_uniform_array_buffers<V: Std140, const N: usize>(
    &mut self,
    f: impl FnOnce(&GPU) -> UniformArrayUpdateContainer<V, N>,
  ) -> Option<UniformBufferDataView<Shader140Array<V, N>>> {
    let (cx, token) =
      self.use_state_with_features(|cx| cx.query_cx.register_multi_updater(f(cx.gpu)));

    if let QueryHookStage::CreateRender { query, .. } = cx.get_stage_mut() {
      query.take_uniform_array_buffer(*token)
    } else {
      None
    }
  }

  fn use_storage_buffer<V: Std430>(
    &mut self,
    f: impl FnOnce(&GPU) -> ReactiveStorageBufferContainer<V>,
  ) -> Option<StorageBufferReadonlyDataView<[V]>> {
    let (cx, token) =
      self.use_state_with_features(|cx| cx.query_cx.register_multi_updater(f(cx.gpu)));

    if let QueryHookStage::CreateRender { query, .. } = cx.get_stage_mut() {
      query.take_storage_array_buffer(*token)
    } else {
      None
    }
  }

  fn use_global_multi_reactive_query<D: ForeignKeySemantic>(
    &mut self,
  ) -> Option<
    Box<dyn DynMultiQuery<Key = EntityHandle<D::ForeignEntity>, Value = EntityHandle<D::Entity>>>,
  > {
    let (cx, token) = self.use_state_with_features(|cx| {
      let query = global_rev_ref().watch_inv_ref::<D>();
      cx.query_cx.register_multi_reactive_query(query)
    });

    if let QueryHookStage::CreateRender { query, .. } = cx.get_stage_mut() {
      query.take_reactive_multi_query_updated(*token)
    } else {
      None
    }
  }

  fn use_reactive_query<K, V, Q>(
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

  fn use_reactive_query_gpu<K, V, Q>(
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

    if let QueryHookStage::CreateRender { query, .. } = cx.get_stage_mut() {
      query.take_reactive_query_updated(*token)
    } else {
      None
    }
  }

  fn use_val_refed_reactive_query<K, V, Q>(
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

    if let QueryHookStage::CreateRender { query, .. } = cx.get_stage_mut() {
      query.take_val_refed_reactive_query_updated(*token)
    } else {
      None
    }
  }

  fn when_render<X>(&self, f: impl FnOnce() -> X) -> Option<X> {
    self.is_in_render().then(f)
  }
  fn is_in_render(&self) -> bool {
    matches!(self.get_stage(), QueryHookStage::CreateRender { .. })
  }
  fn when_init<X>(&self, f: impl FnOnce() -> X) -> Option<X> {
    self.is_creating().then(f)
  }
}

pub struct QueryGPUHookCxImpl<'a> {
  pub memory: &'a mut FunctionMemory,
  pub gpu: &'a GPU,
  pub query_cx: &'a mut ReactiveQueryCtx,
  pub db_linear_changes: &'a mut DBLinearChangeWatchGroup,
  pub task_pool: &'a mut AsyncTaskPool,
  pub stage: QueryHookStage,
  pub task_spawner: &'a TaskSpawner,
}

pub enum QueryHookStage {
  Update,
  CreateRender {
    query: QueryResultCtx,
    task: TaskPoolResultCx,
  },
}

unsafe impl<'a> HooksCxLike for QueryGPUHookCxImpl<'a> {
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
    };
    self.memory.flush(&mut drop_cx as *mut _ as *mut ());
  }
}

impl<'a> QueryGPUHookCx for QueryGPUHookCxImpl<'a> {
  fn get_stage(&self) -> &QueryHookStage {
    &self.stage
  }
  fn get_stage_mut(&mut self) -> &mut QueryHookStage {
    &mut self.stage
  }
  fn get_query_cx(&mut self) -> &mut ReactiveQueryCtx {
    self.query_cx
  }
  fn get_linear_changes(&mut self) -> &mut DBLinearChangeWatchGroup {
    self.db_linear_changes
  }
  fn get_task_pool(&mut self) -> &mut AsyncTaskPool {
    self.task_pool
  }
  fn get_task_spawner(&mut self) -> &TaskSpawner {
    self.task_spawner
  }

  fn gpu(&self) -> &GPU {
    self.gpu
  }

  fn use_state_with_features<T: 'static + for<'x> CanCleanUpFrom<QueryGPUHookDropCx<'x>>>(
    &mut self,
    init: impl FnOnce(QueryGPUHookFeatureCx) -> T,
  ) -> (&mut Self, &mut T) {
    let s = unsafe { std::mem::transmute_copy(&self) };

    let state = self.memory.expect_state_init(
      || {
        init(QueryGPUHookFeatureCx {
          gpu: self.gpu,
          query_cx: self.query_cx,
          linear_changes: self.db_linear_changes,
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
}

struct NothingToDrop<T>(T);

impl<T> CanCleanUpFrom<QueryGPUHookDropCx<'_>> for NothingToDrop<T> {
  fn drop_from_cx(&mut self, _: &mut QueryGPUHookDropCx) {}
}

pub struct QueryGPUHookDropCx<'a> {
  pub query_cx: &'a mut ReactiveQueryCtx,
  pub db_linear_changes: &'a mut DBLinearChangeWatchGroup,
}
