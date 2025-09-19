use crate::*;

pub enum UseResult<T> {
  SpawnStageFuture(Box<dyn Future<Output = T> + Unpin + Send + Sync>),
  SpawnStageReady(T),
  ResolveStageReady(T),
  NotInStage,
}

impl<T: Send + Sync + 'static> UseResult<T> {
  pub fn map_only_spawn_stage_in_thread_dual_query<U: Send + 'static>(
    self,
    cx: &mut impl QueryHookCxLike,
    f: impl FnOnce(T) -> U + Send + Sync + 'static,
  ) -> UseResult<U>
  where
    T: DualQueryLike,
  {
    self.map_only_spawn_stage_in_thread(cx, |q| q.is_change_possible_empty(), f)
  }

  pub fn map_only_spawn_stage_in_thread<U: Send + 'static>(
    self,
    cx: &mut impl QueryHookCxLike,
    should_do_work_in_main_thread: impl FnOnce(&T) -> bool + Send + Sync + 'static,
    f: impl FnOnce(T) -> U + Send + Sync + 'static,
  ) -> UseResult<U> {
    match self {
      UseResult::SpawnStageFuture(fut) => {
        let spawner = cx.spawner().unwrap();
        let spawner = spawner.clone();
        let fut = async move {
          let r = fut.await;
          if should_do_work_in_main_thread(&r) {
            f(r)
          } else {
            spawner.spawn_task(move || f(r)).await
          }
        };
        UseResult::SpawnStageFuture(Box::new(Box::pin(fut)))
      }
      UseResult::SpawnStageReady(t) => UseResult::SpawnStageReady(f(t)),
      UseResult::ResolveStageReady(_) => UseResult::NotInStage,
      UseResult::NotInStage => UseResult::NotInStage,
    }
  }

  pub fn map_only_spawn_stage<U>(
    self,
    f: impl FnOnce(T) -> U + Send + Sync + 'static,
  ) -> UseResult<U> {
    use futures::FutureExt;
    match self {
      UseResult::SpawnStageFuture(fut) => UseResult::SpawnStageFuture(Box::new(fut.map(f))),
      UseResult::SpawnStageReady(t) => UseResult::SpawnStageReady(f(t)),
      UseResult::ResolveStageReady(_) => UseResult::NotInStage,
      UseResult::NotInStage => UseResult::NotInStage,
    }
  }

  /// note, this mapping is map both spawn stage and resolve stage,
  /// so if the T contains changes, the change consuming should not using this method
  /// or the change will be consumed twice and cause logic error
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

    if self.is_spawn_stage_ready() && other.is_spawn_stage_ready() {
      return UseResult::SpawnStageReady((
        self.into_spawn_stage().unwrap(),
        other.into_spawn_stage().unwrap(),
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

impl<T: Send + Sync> UseResult<T> {
  pub fn fork(self) -> (Self, Self)
  where
    T: Clone + 'static,
  {
    match self {
      UseResult::SpawnStageFuture(future) => {
        let future = future.shared();
        let future2 = future.clone();
        (
          UseResult::SpawnStageFuture(Box::new(future)),
          UseResult::SpawnStageFuture(Box::new(future2)),
        )
      }
      UseResult::SpawnStageReady(r) => (
        UseResult::SpawnStageReady(r.clone()),
        UseResult::SpawnStageReady(r),
      ),
      UseResult::ResolveStageReady(r) => (
        UseResult::ResolveStageReady(r.clone()),
        UseResult::ResolveStageReady(r),
      ),
      UseResult::NotInStage => (UseResult::NotInStage, UseResult::NotInStage),
    }
  }

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

  pub fn into_resolve_stage(self) -> Option<T> {
    match self {
      UseResult::ResolveStageReady(t) => Some(t),
      _ => None,
    }
  }

  pub fn into_spawn_stage(self) -> Option<T> {
    match self {
      UseResult::SpawnStageReady(t) => Some(t),
      _ => None,
    }
  }

  pub fn is_resolve_stage(&self) -> bool {
    matches!(self, UseResult::ResolveStageReady(_))
  }

  pub fn is_spawn_stage_ready(&self) -> bool {
    matches!(self, UseResult::SpawnStageReady(_))
  }

  pub fn if_resolve_stage(self) -> Option<T> {
    match self {
      UseResult::ResolveStageReady(t) => Some(t),
      _ => None,
    }
  }

  pub fn if_ready(self) -> Option<T> {
    match self {
      UseResult::SpawnStageReady(t) => Some(t),
      UseResult::ResolveStageReady(t) => Some(t),
      _ => None,
    }
  }

  pub fn expect_resolve_stage(self) -> T {
    self.if_resolve_stage().unwrap()
  }

  pub fn into_spawn_stage_future(
    self,
  ) -> Option<Box<dyn Future<Output = T> + Unpin + Sync + Send>> {
    match self {
      UseResult::SpawnStageFuture(t) => Some(t),
      _ => None,
    }
  }

  pub fn expect_spawn_stage_future(self) -> Box<dyn Future<Output = T> + Unpin + Sync + Send> {
    self.into_spawn_stage_future().unwrap()
  }

  pub fn expect_spawn_stage_ready(self) -> T {
    match self {
      UseResult::SpawnStageReady(t) => t,
      _ => panic!("expect spawn stage ready"),
    }
  }
}

impl<T: Clone + Send + Sync + 'static> UseResult<T> {
  pub fn use_assure_result(self, cx: &mut impl QueryHookCxLike) -> UseResult<T> {
    let (cx, token) = cx.use_plain_state(|| u32::MAX);

    match cx.stage() {
      QueryHookStage::SpawnTask { pool, .. } => {
        if let UseResult::SpawnStageFuture(fut) = self {
          *token = pool.install_task(fut);
          UseResult::NotInStage
        } else {
          *token = u32::MAX;
          self
        }
      }
      QueryHookStage::ResolveTask { task, .. } => {
        if *token != u32::MAX {
          UseResult::ResolveStageReady(task.expect_result_by_id(*token))
        } else {
          self
        }
      }
      _ => UseResult::NotInStage,
    }
  }

  pub fn use_global_shared(self, cx: &mut impl QueryHookCxLike) -> TaskUseResult<T> {
    let future = match self {
      UseResult::SpawnStageFuture(future) => Some(future),
      UseResult::SpawnStageReady(result) => Some(Box::new(futures::future::ready(result))
        as Box<dyn Future<Output = T> + Unpin + Send + Sync>),
      UseResult::ResolveStageReady(_) => return TaskUseResult::NotInStage,
      UseResult::NotInStage => return TaskUseResult::NotInStage,
    };
    cx.use_global_shared_future(future)
  }

  pub fn use_retain_view_to_resolve_stage(self, cx: &mut impl QueryHookCxLike) -> UseResult<T::View>
  where
    T: DualQueryLike,
  {
    let _self = self.use_assure_result(cx);

    let (cx, view_temp) = cx.use_plain_state_default::<Option<T::View>>();

    match _self {
      UseResult::SpawnStageFuture(_) => unreachable!("we have assure the result"),
      UseResult::SpawnStageReady(v) => {
        *view_temp = Some(v.view());
        UseResult::NotInStage
      }
      UseResult::ResolveStageReady(v) => UseResult::ResolveStageReady(v.view()),
      UseResult::NotInStage => {
        if cx.is_resolve_stage() {
          if let Some(v) = view_temp.take() {
            UseResult::ResolveStageReady(v)
          } else {
            UseResult::NotInStage
          }
        } else {
          UseResult::NotInStage
        }
      }
    }
  }
}

impl<T> UseResult<T>
where
  T: DualQueryLike,
{
  pub fn use_validation(
    self,
    cx: &mut impl QueryHookCxLike,
    label: &'static str,
    log_change: bool,
  ) -> UseResult<T> {
    let validator = cx.use_shared_hash_map();
    self.map(move |dual| {
      let (_, d) = dual.view_delta_ref();
      validate_delta(&mut validator.write(), log_change, label, d);

      dual
    })
  }

  pub fn fanout<U: TriQueryLike<Value = T::Key>>(
    self,
    other: UseResult<U>,
    cx: &mut impl QueryHookCxLike,
  ) -> UseResult<
    DualQuery<ChainQuery<U::View, T::View>, Arc<FastHashMap<U::Key, ValueChange<T::Value>>>>,
  > {
    self.join(other).map_only_spawn_stage_in_thread(
      cx,
      |(a, b)| a.is_change_possible_empty() && b.is_change_possible_empty(),
      |(a, b)| a.fanout(b),
    )
  }

  pub fn dual_query_zip<Q>(
    self,
    other: UseResult<Q>,
  ) -> UseResult<impl DualQueryLike<Key = T::Key, Value = (T::Value, Q::Value)>>
  where
    Q: DualQueryLike<Key = T::Key>,
  {
    self.join(other).map(|(a, b)| a.dual_query_zip(b))
  }

  pub fn dual_query_intersect<Q>(
    self,
    other: UseResult<Q>,
  ) -> UseResult<impl DualQueryLike<Key = T::Key, Value = (T::Value, Q::Value)>>
  where
    Q: DualQueryLike<Key = T::Key>,
  {
    self.join(other).map(|(a, b)| a.dual_query_intersect(b))
  }

  pub fn dual_query_filter_by_set<Q>(
    self,
    other: UseResult<Q>,
  ) -> UseResult<impl DualQueryLike<Key = T::Key, Value = T::Value>>
  where
    Q: DualQueryLike<Key = T::Key>,
  {
    self.join(other).map(|(a, b)| a.dual_query_filter_by_set(b))
  }

  pub fn use_dual_query_hash_many_to_one(
    self,
    cx: &mut impl QueryHookCxLike,
  ) -> UseResult<impl TriQueryLike<Key = T::Key, Value = T::Value>>
  where
    T::Value: CKey,
  {
    let map = cx.use_shared_hash_map();

    self.map_only_spawn_stage_in_thread_dual_query(cx, move |t| {
      let (view, delta) = t.view_delta();
      bookkeeping_hash_relation(&mut map.write(), &delta);

      TriQuery {
        base: DualQuery { view, delta },
        rev_many_view: map.make_read_holder(),
      }
    })
  }

  pub fn use_dual_query_hash_reverse_checked_one_one(
    self,
    cx: &mut impl QueryHookCxLike,
  ) -> UseResult<impl DualQueryLike<Key = T::Value, Value = T::Key>>
  where
    T::Value: CKey,
  {
    let map = cx.use_shared_hash_map();

    self.map_only_spawn_stage_in_thread_dual_query(cx, move |t| {
      let mut mapping = map.write();
      let mut mutations = FastHashMap::<T::Value, ValueChange<T::Key>>::default();
      use std::ops::DerefMut;
      let mut mutator = QueryMutationCollector {
        delta: &mut mutations,
        target: mapping.deref_mut(),
      };

      for (k, change) in t.delta().iter_key_value() {
        match change {
          ValueChange::Delta(v, pv) => {
            if let Some(pv) = &pv {
              mutator.remove(pv.clone());
            }

            if let Some(previous) = mutator.set_value(v.clone(), k.clone()) {
              panic!("one to one relation assertion failed, value: {:?} checked has previous mapping key {:?}, when receive new mapping key {:?}", v, previous, k);
            }

          }
          ValueChange::Remove(pv) => {
            mutator.remove(pv);
          }
        }
      }
      drop(mapping);

      DualQuery {
        view: map.make_read_holder(),
        delta: Arc::new(mutations),
      }
    })
  }

  pub fn dual_query_filter(
    self,
    f: impl Fn(T::Value) -> bool + Clone + Sync + Send + 'static,
  ) -> UseResult<impl DualQueryLike<Key = T::Key, Value = T::Value>> {
    self.map(|t| t.dual_query_filter(f))
  }

  pub fn dual_query_filter_map<V2: CValue>(
    self,
    f: impl Fn(T::Value) -> Option<V2> + Clone + Sync + Send + 'static,
  ) -> UseResult<impl DualQueryLike<Key = T::Key, Value = V2>> {
    self.map(|t| t.dual_query_filter_map(f))
  }

  pub fn dual_query_map<V2: CValue>(
    self,
    f: impl Fn(T::Value) -> V2 + Clone + Sync + Send + 'static,
  ) -> UseResult<impl DualQueryLike<Key = T::Key, Value = V2>> {
    self.map(|t| t.dual_query_map(f))
  }

  pub fn use_dual_query_execute_map<V2, F, FF>(
    self,
    cx: &mut impl QueryHookCxLike,
    f: F,
  ) -> UseResult<
    DualQuery<
      LockReadGuardHolder<FastHashMap<T::Key, V2>>,
      Arc<FastHashMap<T::Key, ValueChange<V2>>>,
    >,
  >
  where
    V2: CValue,
    F: FnOnce() -> FF + Send + Sync + 'static,
    FF: FnMut(&T::Key, T::Value) -> V2 + Send + Sync + 'static,
  {
    let cache = cx.use_shared_hash_map();

    self.map_only_spawn_stage_in_thread_dual_query(cx, move |t| {
      let d = t.delta();
      let materialized = d.iter_key_value().collect::<Vec<_>>();

      // map_creator call or drop may have significant cost, so we only create mapper
      // if we have actual delta processing to do.
      let d = if !materialized.is_empty() {
        let mut mapper = f();
        let mut cache = cache.write();
        let materialized: FastHashMap<T::Key, ValueChange<V2>> = materialized
          .into_iter()
          .map(|(k, delta)| match delta {
            ValueChange::Delta(d, _p) => {
              let new_value = mapper(&k, d);
              let p = cache.insert(k.clone(), new_value.clone());
              (k, ValueChange::Delta(new_value, p))
            }
            ValueChange::Remove(_p) => {
              let p = cache.remove(&k).unwrap();
              (k, ValueChange::Remove(p))
            }
          })
          .collect();
        Arc::new(materialized)
      } else {
        Default::default()
      };

      let v = cache.make_read_holder();

      DualQuery { view: v, delta: d }
    })
  }

  pub fn dual_query_select<Q: DualQueryLike<Key = T::Key, Value = T::Value>>(
    self,
    other: UseResult<Q>,
  ) -> UseResult<impl DualQueryLike<Key = T::Key, Value = T::Value>> {
    self.join(other).map(|(a, b)| a.dual_query_select(b))
  }

  pub fn dual_query_union<Q: DualQueryLike<Key = T::Key>, O: CValue>(
    self,
    other: UseResult<Q>,
    f: impl Fn((Option<T::Value>, Option<Q::Value>)) -> Option<O> + Copy + Sync + Send + 'static,
  ) -> UseResult<impl DualQueryLike<Key = T::Key, Value = O>> {
    self.join(other).map(move |(a, b)| a.dual_query_union(b, f))
  }

  pub fn dual_query_boxed(self) -> UseResult<BoxedDynDualQuery<T::Key, T::Value>> {
    self.map(|v| {
      let (a, d) = v.view_delta();
      DualQuery {
        view: a.into_boxed(),
        delta: d.into_boxed(),
      }
    })
  }

  pub fn into_delta_change(self) -> UseResult<DeltaQueryAsChange<T::Delta>> {
    self.map(|v| v.view_delta().1.into_change())
  }
}
