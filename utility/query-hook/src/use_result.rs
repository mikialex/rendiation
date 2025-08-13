use crate::*;

pub enum UseResult<T> {
  SpawnStageFuture(Box<dyn Future<Output = T> + Unpin + Send + Sync>),
  SpawnStageReady(T),
  ResolveStageReady(T),
  NotInStage,
}

impl<T> From<TaskUseResult<T>> for UseResult<T> {
  fn from(value: TaskUseResult<T>) -> Self {
    match value {
      TaskUseResult::SpawnId(_) => UseResult::NotInStage,
      TaskUseResult::Result(r) => UseResult::ResolveStageReady(r),
    }
  }
}

impl<T: Send + Sync + 'static> UseResult<T> {
  pub fn clone_except_future(&self) -> Self
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

  pub fn into_future(self) -> Option<Box<dyn Future<Output = T> + Unpin + Send + Sync>> {
    match self {
      UseResult::SpawnStageFuture(future) => Some(future),
      UseResult::SpawnStageReady(r) => {
        let future = std::future::ready(r);
        Some(Box::new(future))
      }
      _ => None,
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

    match (a, b) {
      (Some(a), Some(b)) => UseResult::SpawnStageFuture(Box::new(futures::future::join(a, b))),
      (None, None) => UseResult::NotInStage,
      _ => panic!("join source corrupted"),
    }
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

  pub fn if_spawn_stage_future(self) -> Option<Box<dyn Future<Output = T> + Unpin + Sync + Send>> {
    match self {
      UseResult::SpawnStageFuture(t) => Some(t),
      _ => None,
    }
  }

  pub fn expect_spawn_stage_future(self) -> Box<dyn Future<Output = T> + Unpin + Sync + Send> {
    self.if_spawn_stage_future().unwrap()
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

impl<T> UseResult<T>
where
  T: DualQueryLike,
{
  pub fn fanout<U: TriQueryLike<Value = T::Key>>(
    self,
    other: UseResult<U>,
  ) -> UseResult<impl DualQueryLike<Key = U::Key, Value = T::Value>> {
    self.join(other).map(|(a, b)| a.fanout(b))
  }

  pub fn into_delta_change(self) -> UseResult<DeltaQueryAsChange<T::Delta>> {
    self.map(|v| v.view_delta().1.into_change())
  }
}
