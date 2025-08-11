use crate::*;

pub enum UseResult<T> {
  SpawnStageFuture(Box<dyn Future<Output = T> + Unpin + Send>),
  SpawnStageReady(T),
  ResolveStageReady(T),
  NotInStage,
}

impl<T: Send + 'static> UseResult<T> {
  pub fn map<U>(self, f: impl FnOnce(T) -> U + Send + 'static) -> UseResult<U> {
    use futures::FutureExt;
    match self {
      UseResult::SpawnStageFuture(fut) => UseResult::SpawnStageFuture(Box::new(fut.map(f))),
      UseResult::SpawnStageReady(t) => UseResult::SpawnStageReady(f(t)),
      UseResult::ResolveStageReady(t) => UseResult::ResolveStageReady(f(t)),
      UseResult::NotInStage => UseResult::NotInStage,
    }
  }

  pub fn into_future(self) -> Box<dyn Future<Output = T> + Unpin + Send> {
    match self {
      UseResult::SpawnStageFuture(future) => future,
      UseResult::SpawnStageReady(r) => {
        let future = std::future::ready(r);
        Box::new(future)
      }
      _ => panic!("stage not match"),
    }
  }

  pub fn join<U: Send + 'static>(self, other: UseResult<U>) -> UseResult<(T, U)> {
    if self.is_resolve_stage() && other.is_resolve_stage() {
      return UseResult::ResolveStageReady((
        self.into_resolve_stage().unwrap(),
        other.into_resolve_stage().unwrap(),
      ));
    }

    let a = self.into_future();
    let b = other.into_future();

    UseResult::SpawnStageFuture(Box::new(futures::future::join(a, b)))
  }

  pub fn into_resolve_stage(self) -> Option<T> {
    match self {
      UseResult::ResolveStageReady(t) => Some(t),
      _ => None,
    }
  }

  pub fn is_resolve_stage(&self) -> bool {
    matches!(self, UseResult::ResolveStageReady(_))
  }

  pub fn expect_resolve_stage(self) -> T {
    match self {
      UseResult::ResolveStageReady(t) => t,
      _ => panic!("expect spawn stage ready"),
    }
  }

  pub fn expect_spawn_stage_future(self) -> Box<dyn Future<Output = T> + Unpin + Send> {
    match self {
      UseResult::SpawnStageFuture(t) => t,
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

pub type UniformBufferCollectionRaw<K, T> = FastHashMap<K, UniformBufferDataView<T>>;
pub type UniformBufferCollection<K, T> = Arc<RwLock<FastHashMap<K, UniformBufferDataView<T>>>>;

pub trait DataChangeGPUExt {
  fn update_uniforms<K: LinearIdentification + CKey, U: Std140 + Default>(
    &self,
    uniforms: &UniformBufferCollection<K, U>,
    offset: usize,
    gpu: &GPU,
  );

  fn update_storage_array<U: Std430 + Default>(
    &self,
    storage: &mut CommonStorageBufferImpl<U>,
    field_offset: usize,
  );
}

impl<T: DataChangeGPUExt> DataChangeGPUExt for UseResult<T> {
  fn update_uniforms<K: LinearIdentification + CKey, U: Std140 + Default>(
    &self,
    uniforms: &UniformBufferCollection<K, U>,
    offset: usize,
    gpu: &GPU,
  ) {
    let r = match self {
      UseResult::SpawnStageReady(r) => r,
      UseResult::ResolveStageReady(r) => r,
      _ => return,
    };
    r.update_uniforms(uniforms, offset, gpu);
  }

  fn update_storage_array<U: Std430 + Default>(
    &self,
    storage: &mut CommonStorageBufferImpl<U>,
    field_offset: usize,
  ) {
    let r = match self {
      UseResult::SpawnStageReady(r) => r,
      UseResult::ResolveStageReady(r) => r,
      _ => return,
    };
    r.update_storage_array(storage, field_offset);
  }
}

impl<T, X> DataChangeGPUExt for X
where
  T: Pod,
  X: DataChanges<Key = u32, Value = T>,
{
  fn update_uniforms<K: LinearIdentification + CKey, U: Std140 + Default>(
    &self,
    uniforms: &UniformBufferCollection<K, U>,
    offset: usize,
    gpu: &GPU,
  ) {
    if self.has_change() {
      let mut uniform = uniforms.write();
      for id in self.iter_removed() {
        uniform.remove(&K::from_alloc_index(id));
      }

      for (id, value) in self.iter_update_or_insert() {
        let buffer = uniform
          .entry(K::from_alloc_index(id))
          .or_insert_with(|| UniformBufferDataView::create_default(&gpu.device));
        // todo, here we should do sophisticated optimization to merge the adjacent writes.
        buffer.write_at(&gpu.queue, &value, offset as u64);
      }
    }
  }

  fn update_storage_array<U: Std430 + Default>(
    &self,
    storage: &mut CommonStorageBufferImpl<U>,
    field_offset: usize,
  ) {
    if self.has_change() {
      for (id, value) in self.iter_update_or_insert() {
        unsafe {
          storage
            .set_value_sub_bytes(id, field_offset, bytes_of(&value))
            .unwrap();
        }
      }
    }

    //
  }
}
