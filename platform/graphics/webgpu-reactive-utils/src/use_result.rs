use crate::*;

pub enum UseResult<T> {
  SpawnStageFuture(Box<dyn Future<Output = T> + Unpin + Send + Sync>),
  SpawnStageReady(T),
  ResolveStageReady(T),
  NotInStage,
}

impl<T: Send + Sync + 'static> UseResult<T> {
  pub fn clone_expect_none_future(&self) -> Self
  where
    T: Clone,
  {
    match self {
      UseResult::SpawnStageFuture(_) => panic!("can not clone future"),
      UseResult::SpawnStageReady(r) => UseResult::SpawnStageReady(r.clone()),
      UseResult::ResolveStageReady(r) => UseResult::ResolveStageReady(r.clone()),
      UseResult::NotInStage => UseResult::NotInStage,
    }
  }

  pub fn map<U>(self, f: impl FnOnce(T) -> U + Send + Sync + 'static) -> UseResult<U> {
    use futures::FutureExt;
    match self {
      UseResult::SpawnStageFuture(fut) => UseResult::SpawnStageFuture(Box::new(fut.map(f))),
      UseResult::SpawnStageReady(t) => UseResult::SpawnStageReady(f(t)),
      UseResult::ResolveStageReady(t) => UseResult::ResolveStageReady(f(t)),
      UseResult::NotInStage => UseResult::NotInStage,
    }
  }

  pub fn into_future(self) -> Box<dyn Future<Output = T> + Unpin + Send + Sync> {
    match self {
      UseResult::SpawnStageFuture(future) => future,
      UseResult::SpawnStageReady(r) => {
        let future = std::future::ready(r);
        Box::new(future)
      }
      _ => panic!("stage not match"),
    }
  }

  pub fn join<U: Send + Sync + 'static>(self, other: UseResult<U>) -> UseResult<(T, U)> {
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

  pub fn if_resolve_stage(self) -> Option<T> {
    match self {
      UseResult::ResolveStageReady(t) => Some(t),
      _ => None,
    }
  }

  pub fn expect_resolve_stage(self) -> T {
    self.if_resolve_stage().unwrap()
  }

  pub fn expect_spawn_stage_future(self) -> Box<dyn Future<Output = T> + Unpin + Sync + Send> {
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

impl<T, U> UseResult<DualQuery<T, U>> {
  pub fn fanout<KMany, KOne, V, X, Y, Z>(
    self,
    other: UseResult<TriQuery<X, Y, Z>>,
  ) -> UseResult<DualQuery<ChainQuery<X, T>, Arc<FastHashMap<KMany, ValueChange<V>>>>>
  where
    KMany: CKey,
    KOne: CKey,
    V: CKey,
    T: Query<Key = KOne, Value = V> + Clone + Send + Sync + 'static,
    U: Query<Key = KOne, Value = ValueChange<V>> + Clone + Send + Sync + 'static,
    X: Query<Key = KMany, Value = KOne> + Send + Clone + Sync + 'static,
    Y: Query<Key = KMany, Value = ValueChange<KOne>> + Clone + Send + Sync + 'static,
    Z: MultiQuery<Key = KOne, Value = KMany> + Clone + Send + Sync + 'static,
  {
    self.join(other).map(|(a, b)| a.compute_fanout(b))
  }
}

pub type UniformBufferCollectionRaw<K, T> = FastHashMap<K, UniformBufferDataView<T>>;
pub type UniformBufferCollection<K, T> = Arc<RwLock<FastHashMap<K, UniformBufferDataView<T>>>>;

pub trait DataChangeGPUExt<K: LinearIdentified + CKey> {
  fn update_uniforms<U: Std140 + Default>(
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

impl<K: LinearIdentified + CKey, T: DataChangeGPUExt<K>> DataChangeGPUExt<K> for UseResult<T> {
  fn update_uniforms<U: Std140 + Default>(
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

impl<K, T, X> DataChangeGPUExt<K> for X
where
  T: Pod,
  K: LinearIdentified + CKey,
  X: DataChanges<Key = K, Value = T>,
{
  fn update_uniforms<U: Std140 + Default>(
    &self,
    uniforms: &UniformBufferCollection<K, U>,
    offset: usize,
    gpu: &GPU,
  ) {
    if self.has_change() {
      let mut uniform = uniforms.write();
      for id in self.iter_removed() {
        uniform.remove(&id);
      }

      for (id, value) in self.iter_update_or_insert() {
        let buffer = uniform
          .entry(id)
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
            .set_value_sub_bytes(id.alloc_index(), field_offset, bytes_of(&value))
            .unwrap();
        }
      }
    }

    //
  }
}
