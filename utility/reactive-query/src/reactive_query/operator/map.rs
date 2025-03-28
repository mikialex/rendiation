use crate::*;

impl<T, F, V2> ReactiveQuery for MappedQuery<T, F>
where
  V2: CValue,
  F: Fn(&T::Key, T::Value) -> V2 + Clone + Send + Sync + 'static,
  T: ReactiveQuery,
{
  type Key = T::Key;
  type Value = V2;
  type Compute = MappedQuery<T::Compute, F>;

  fn describe(&self, cx: &mut Context) -> Self::Compute {
    MappedQuery {
      base: self.base.describe(cx),
      mapper: self.mapper.clone(),
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.base.request(request)
  }
}

impl<T, F, V2> QueryCompute for MappedQuery<T, F>
where
  V2: CValue,
  F: Fn(&T::Key, T::Value) -> V2 + Clone + Send + Sync + 'static,
  T: QueryCompute,
{
  type Key = T::Key;
  type Value = V2;

  type Changes = MappedQuery<T::Changes, ValueChangeMapper<F>>;
  type View = MappedQuery<T::View, F>;

  fn resolve(&mut self, cx: &QueryResolveCtx) -> (Self::Changes, Self::View) {
    let (d, v) = self.base.resolve(cx);
    let mapper = self.mapper.clone();
    let d = d.map(ValueChangeMapper(mapper));
    let v = v.map(self.mapper.clone());

    (d, v)
  }
}

#[derive(Clone, Copy)]
pub struct ValueChangeMapper<F>(pub F);
impl<K, V, V2, F: Fn(&K, V) -> V2 + Clone> FnOnce<(&K, ValueChange<V>)> for ValueChangeMapper<F> {
  type Output = ValueChange<V2>;

  extern "rust-call" fn call_once(self, args: (&K, ValueChange<V>)) -> Self::Output {
    self.call(args)
  }
}

impl<K, V, V2, F: Fn(&K, V) -> V2 + Clone> FnMut<(&K, ValueChange<V>)> for ValueChangeMapper<F> {
  extern "rust-call" fn call_mut(&mut self, args: (&K, ValueChange<V>)) -> Self::Output {
    self.call(args)
  }
}

impl<K, V, V2, F: Fn(&K, V) -> V2 + Clone> Fn<(&K, ValueChange<V>)> for ValueChangeMapper<F> {
  extern "rust-call" fn call(&self, (k, v): (&K, ValueChange<V>)) -> Self::Output {
    v.map(|v| (self.0.clone())(k, v))
  }
}

impl<T, F, V2> AsyncQueryCompute for MappedQuery<T, F>
where
  V2: CValue,
  F: Fn(&T::Key, T::Value) -> V2 + Clone + Send + Sync + 'static,
  T: AsyncQueryCompute,
{
  type Task = impl Future<Output = (Self::Changes, Self::View)>;

  fn create_task(&mut self, cx: &mut AsyncQueryCtx) -> Self::Task {
    let mapper = self.mapper.clone();
    let c = cx.resolve_cx().clone();
    self
      .base
      .create_task(cx)
      .map(move |base| MappedQuery { base, mapper }.resolve(&c))
  }
}

impl<F1, F2, T, K2> ReactiveQuery for KeyDualMappedQuery<T, F1, F2>
where
  K2: CKey,
  F1: Fn(T::Key) -> K2 + Copy + Send + Sync + 'static,
  F2: Fn(K2) -> T::Key + Copy + Send + Sync + 'static,
  T: ReactiveQuery,
{
  type Key = K2;
  type Value = T::Value;
  type Compute = KeyDualMappedQuery<T::Compute, F1, F2>;

  fn describe(&self, cx: &mut Context) -> Self::Compute {
    KeyDualMappedQuery {
      f1: self.f1,
      f2: self.f2,
      base: self.base.describe(cx),
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.base.request(request)
  }
}

impl<F1, F2, T, K2> QueryCompute for KeyDualMappedQuery<T, F1, F2>
where
  K2: CKey,
  F1: Fn(T::Key) -> K2 + Copy + Send + Sync + 'static,
  F2: Fn(K2) -> T::Key + Copy + Send + Sync + 'static,
  T: QueryCompute,
{
  type Key = K2;
  type Value = T::Value;

  type Changes = KeyDualMappedQuery<T::Changes, F1, AutoSomeFnResult<F2>>;
  type View = KeyDualMappedQuery<T::View, F1, AutoSomeFnResult<F2>>;

  fn resolve(&mut self, cx: &QueryResolveCtx) -> (Self::Changes, Self::View) {
    let (d, v) = self.base.resolve(cx);
    let d = d.key_dual_map(self.f1, self.f2);
    let v = v.key_dual_map(self.f1, self.f2);
    (d, v)
  }
}
impl<F1, F2, T, K2> AsyncQueryCompute for KeyDualMappedQuery<T, F1, F2>
where
  K2: CKey,
  F1: Fn(T::Key) -> K2 + Copy + Send + Sync + 'static,
  F2: Fn(K2) -> T::Key + Copy + Send + Sync + 'static,
  T: AsyncQueryCompute,
{
  type Task = impl Future<Output = (Self::Changes, Self::View)>;

  fn create_task(&mut self, cx: &mut AsyncQueryCtx) -> Self::Task {
    let f1 = self.f1;
    let f2 = self.f2;
    let c = cx.resolve_cx().clone();
    self
      .base
      .create_task(cx)
      .map(move |base| KeyDualMappedQuery { base, f1, f2 }.resolve(&c))
  }
}

/// A map operator that internal contains a materialization and a mapper creator logic.
///
/// Compare to [MappedQuery], this map will not compute previous delta to improve performance
/// at cost of memory usage. Use this if your mapper fn is expensive to run.
///
/// The mapper creator will create the mapper at the computation start time, collecting some expensive
/// operations such as lock access or context creation in advance to improve mapping performance.
pub struct MapExecution<T, K, F, V2> {
  pub inner: T,
  pub map_creator: F,
  pub cache: Arc<RwLock<FastHashMap<K, V2>>>,
}

impl<T, F, V2, FF> ReactiveQuery for MapExecution<T, T::Key, F, V2>
where
  F: Fn() -> FF + Send + Sync + Clone + 'static,
  FF: FnMut(&T::Key, T::Value) -> V2 + Send + Sync + 'static,
  V2: CValue,
  T: ReactiveQuery,
{
  type Key = T::Key;
  type Value = V2;
  type Compute = MapExecution<T::Compute, T::Key, F, V2>;

  fn describe(&self, cx: &mut Context) -> Self::Compute {
    MapExecution {
      inner: self.inner.describe(cx),
      map_creator: self.map_creator.clone(),
      cache: self.cache.clone(),
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    match request {
      ReactiveQueryRequest::MemoryShrinkToFit => self.cache.write().shrink_to_fit(),
    }
    self.inner.request(request)
  }
}

impl<T, F, V2, FF> QueryCompute for MapExecution<T, T::Key, F, V2>
where
  F: Fn() -> FF + Send + Sync + 'static,
  FF: FnMut(&T::Key, T::Value) -> V2 + Send + Sync + 'static,
  V2: CValue,
  T: QueryCompute,
{
  type Key = T::Key;
  type Value = V2;

  type Changes = Arc<FastHashMap<T::Key, ValueChange<V2>>>;
  type View = LockReadGuardHolder<FastHashMap<T::Key, V2>>;

  fn resolve(&mut self, cx: &QueryResolveCtx) -> (Self::Changes, Self::View) {
    let (d, v) = self.inner.resolve(cx);
    cx.keep_view_alive(v);

    let mut mapper = (self.map_creator)();
    let materialized = d.iter_key_value().collect::<Vec<_>>();
    let mut cache = self.cache.write();
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
    let d = Arc::new(materialized);
    drop(cache);

    let v = self.cache.make_read_holder();
    (d, v)
  }
}

impl<T, F, V2, FF> AsyncQueryCompute for MapExecution<T, T::Key, F, V2>
where
  F: Fn() -> FF + Clone + Send + Sync + 'static,
  FF: FnMut(&T::Key, T::Value) -> V2 + Send + Sync + 'static,
  V2: CValue,
  T: AsyncQueryCompute,
{
  type Task = impl Future<Output = (Self::Changes, Self::View)>;

  fn create_task(&mut self, cx: &mut AsyncQueryCtx) -> Self::Task {
    let map_creator = self.map_creator.clone();
    let cache = self.cache.clone();

    let inner = self.inner.create_task(cx);

    let f = cx.then_spawn(inner, move |inner, cx| {
      MapExecution {
        inner,
        map_creator,
        cache,
      }
      .resolve(cx)
    });

    avoid_huge_debug_symbols_by_boxing_future(f)
  }
}
