use ::hook::*;
use database::*;

use crate::*;

pub trait QueryGPUHookCx: HooksCxLike {
  fn gpu(&self) -> &GPU;
  fn get_stage(&self) -> &QueryHookStage;
  fn get_stage_mut(&mut self) -> &mut QueryHookStage;

  fn use_begin_change_set_collect(&mut self) -> (&mut Self, impl FnOnce() -> Option<bool>);

  fn use_gpu_query_init<T: 'static>(
    &mut self,
    init: impl FnOnce(&GPU, &mut ReactiveQueryCtx) -> T,
  ) -> (&mut Self, &mut T, &mut QueryHookStage);

  fn use_state<T: Default + 'static>(&mut self) -> (&mut Self, &mut T) {
    self.use_state_init(T::default)
  }

  fn use_state_init<T: 'static>(&mut self, init: impl FnOnce() -> T) -> (&mut Self, &mut T) {
    let (cx, state, _) = self.use_gpu_query_init(|_, _| init());
    (cx, state)
  }

  fn use_gpu_init<T: 'static>(&mut self, init: impl FnOnce(&GPU) -> T) -> (&mut Self, &mut T) {
    let (cx, state, _) = self.use_gpu_query_init(|gpu, _| init(gpu));
    (cx, state)
  }

  fn use_multi_updater_gpu<T: 'static>(
    &mut self,
    f: impl FnOnce(&GPU) -> MultiUpdateContainer<T>,
  ) -> Option<LockReadGuardHolder<MultiUpdateContainer<T>>> {
    let (_, token, stage) =
      self.use_gpu_query_init(|gpu, query_cx| query_cx.register_multi_updater(f(gpu)));

    if let QueryHookStage::Render(results) = stage {
      results.take_multi_updater_updated::<T>(*token)
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
    let (_, token, stage) =
      self.use_gpu_query_init(|gpu, query_cx| query_cx.register_typed(f(gpu)));

    if let QueryHookStage::Render(results) = stage {
      Some(
        *results
          .take_result(*token)
          .unwrap()
          .downcast::<T::Output>()
          .unwrap(),
      )
    } else {
      None
    }
  }

  fn use_uniform_buffers<K, V: Std140 + 'static>(
    &mut self,
    f: impl FnOnce(UniformUpdateContainer<K, V>, &GPU) -> UniformUpdateContainer<K, V>,
  ) -> Option<LockReadGuardHolder<UniformUpdateContainer<K, V>>> {
    let (_, token, stage) = self.use_gpu_query_init(|gpu, query_cx| {
      let source = UniformUpdateContainer::<K, V>::default();
      query_cx.register_multi_updater(f(source, gpu))
    });

    if let QueryHookStage::Render(results) = stage {
      todo!()
    } else {
      None
    }
  }

  fn use_uniform_array_buffers<V: Std140, const N: usize>(
    &mut self,
    source: impl FnOnce(&GPU) -> UniformArrayUpdateContainer<V, N>,
  ) -> Option<UniformBufferDataView<Shader140Array<V, N>>> {
    // let (_, token, stage) = self.use_gpu_query_init(|gpu, query_cx| {
    //   // let source = UniformUpdateContainer::<K, V>::default();
    //   // query_cx.register(f(source, gpu))
    // });

    // if let QueryHookStage::Render(results) = stage {
    //   todo!()
    // } else {
    //   None
    // }
    todo!()
  }

  fn use_storage_buffer<V: Std430>(
    &mut self,
    source: impl FnOnce(&GPU) -> ReactiveStorageBufferContainer<V>,
  ) -> Option<StorageBufferReadonlyDataView<[V]>> {
    todo!()
  }

  fn use_global_multi_reactive_query<D: ForeignKeySemantic>(
    &mut self,
  ) -> Option<
    Box<dyn DynMultiQuery<Key = EntityHandle<D::ForeignEntity>, Value = EntityHandle<D::Entity>>>,
  > {
    let (_, token, stage) = self.use_gpu_query_init(|_, query_cx| {
      let query = global_rev_ref().watch_inv_ref::<D>();
      query_cx.register_multi_reactive_query(query)
    });

    if let QueryHookStage::Render(results) = stage {
      results.take_reactive_multi_query_updated(*token)
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
    let (_, token, stage) =
      self.use_gpu_query_init(|gpu, query_cx| query_cx.register_reactive_query(f(gpu)));

    if let QueryHookStage::Render(results) = stage {
      results.take_reactive_query_updated(*token)
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
    let (_, token, stage) =
      self.use_gpu_query_init(|gpu, query_cx| query_cx.register_val_refed_reactive_query(f(gpu)));

    if let QueryHookStage::Render(results) = stage {
      results.take_val_refed_reactive_query_updated(*token)
    } else {
      None
    }
  }

  fn when_render<X>(&self, f: impl FnOnce() -> X) -> Option<X> {
    self.is_in_render().then_some(f())
  }
  fn is_in_render(&self) -> bool {
    matches!(self.get_stage(), QueryHookStage::Render(..))
  }
  fn when_init<X>(&self, f: impl FnOnce() -> X) -> Option<X> {
    matches!(self.get_stage(), QueryHookStage::Init).then_some(f())
  }
}

pub struct QueryGPUHookCxImpl<'a> {
  pub memory: &'a mut FunctionMemory,
  pub gpu: &'a GPU,
  pub query_cx: &'a mut ReactiveQueryCtx,
  pub stage: QueryHookStage,
}

pub enum QueryHookStage {
  Init,
  UnInit,
  Render(QueryResultCtx),
}

unsafe impl<'a> HooksCxLike for QueryGPUHookCxImpl<'a> {
  fn memory_mut(&mut self) -> &mut FunctionMemory {
    self.memory
  }

  fn memory_ref(&self) -> &FunctionMemory {
    self.memory
  }

  fn flush(&mut self) {
    todo!()
  }
}

impl<'a> QueryGPUHookCx for QueryGPUHookCxImpl<'a> {
  fn get_stage(&self) -> &QueryHookStage {
    &self.stage
  }
  fn get_stage_mut(&mut self) -> &mut QueryHookStage {
    &mut self.stage
  }
  fn use_begin_change_set_collect(&mut self) -> (&mut Self, impl FnOnce() -> Option<bool>) {
    (self, || todo!())
  }

  fn gpu(&self) -> &GPU {
    self.gpu
  }

  fn use_gpu_query_init<T: 'static>(
    &mut self,
    init: impl FnOnce(&GPU, &mut ReactiveQueryCtx) -> T,
  ) -> (&mut Self, &mut T, &mut QueryHookStage) {
    let state = self
      .memory
      .expect_state_init(|| init(self.gpu, self.query_cx), |_: &mut T, _: &mut ()| {});
    todo!()
  }
}
