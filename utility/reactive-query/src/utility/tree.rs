use crate::*;

pub type ReactiveTreeConnectivity<K> = BoxedDynReactiveOneToManyRelation<K, K>;
pub type ReactiveTreePayload<K, T> = BoxedDynReactiveQuery<K, T>;

pub fn tree_payload_derive_by_parent_decide_children<K, T>(
  connectivity: ReactiveTreeConnectivity<K>,
  payload: ReactiveTreePayload<K, T>,
  derive_logic: impl Fn(&T, Option<&T>) -> T + Send + Sync + 'static + Copy,
) -> impl ReactiveQuery<Key = K, Value = T>
where
  K: CKey,
  T: CValue,
{
  TreeDerivedData {
    data: Default::default(),
    payload_source: payload,
    connectivity_source: connectivity,
    derive_logic,
  }
}

struct TreeDerivedData<K, T, F> {
  /// where the actually derived data stored
  data: Arc<RwLock<FastHashMap<K, T>>>,
  payload_source: ReactiveTreePayload<K, T>,
  connectivity_source: ReactiveTreeConnectivity<K>,
  derive_logic: F,
}

struct TreeDerivedDataCompute<K, T, F, S, C> {
  data: Arc<RwLock<FastHashMap<K, T>>>,
  derive_logic: F,
  payload_source: S,
  connectivity_source: C,
}

impl<K, T, F, S, C> AsyncQueryCompute for TreeDerivedDataCompute<K, T, F, S, C>
where
  K: CKey,
  T: CValue,
  S: AsyncQueryCompute<Key = K, Value = T>,
  C: AsyncQueryCompute<Key = K, Value = K, View: MultiQuery<Key = K, Value = K>>,
  F: Fn(&T, Option<&T>) -> T + Send + Sync + 'static + Copy,
{
  fn create_task(
    &mut self,
    cx: &mut AsyncQueryCtx,
  ) -> QueryComputeTask<(Self::Changes, Self::View)> {
    let derive_logic = self.derive_logic;
    let data = self.data.clone();
    let payload_source = self.payload_source.create_task(cx);
    let connectivity_source = self.connectivity_source.create_task(cx);
    let joined = futures::future::join(payload_source, connectivity_source);
    cx.then_spawn_compute(joined, move |(payload_source, connectivity_source)| {
      TreeDerivedDataCompute {
        data: data.clone(),
        derive_logic,
        payload_source,
        connectivity_source,
      }
    })
    .into_boxed_future()
  }
}

impl<K, T, F, S, C> QueryCompute for TreeDerivedDataCompute<K, T, F, S, C>
where
  K: CKey,
  T: CValue,
  S: QueryCompute<Key = K, Value = T>,
  C: QueryCompute<Key = K, Value = K, View: MultiQuery<Key = K, Value = K>>,
  F: Fn(&T, Option<&T>) -> T + Send + Sync + 'static + Copy,
{
  type Key = K;
  type Value = T;
  type Changes = Arc<FastHashMap<K, ValueChange<T>>>;
  type View = LockReadGuardHolder<FastHashMap<K, T>>;

  fn resolve(&mut self, cx: &QueryResolveCtx) -> (Self::Changes, Self::View) {
    let (payload_change, current_source) = self.payload_source.resolve(cx);
    let (connectivity_change, current_connectivity) = self.connectivity_source.resolve(cx);
    let current_inv_connectivity = current_connectivity.clone();

    let mut derive_tree = self.data.write();

    let derive_changes = compute_tree_derive(
      &mut derive_tree,
      self.derive_logic,
      current_source,
      payload_change,
      current_connectivity,
      current_inv_connectivity,
      connectivity_change,
    );

    drop(derive_tree);
    let d = Arc::new(derive_changes);
    let v = self.data.make_read_holder();
    (d, v)
  }
}

impl<K, T, F> ReactiveQuery for TreeDerivedData<K, T, F>
where
  K: CKey,
  T: CValue,
  F: Fn(&T, Option<&T>) -> T + Send + Sync + 'static + Copy,
{
  type Key = K;
  type Value = T;
  // how can we improve this??
  type Compute = TreeDerivedDataCompute<
    K,
    T,
    F,
    BoxedDynQueryCompute<K, T>,
    BoxedDynOneToManyQueryCompute<K, K>,
  >;

  fn describe(&self, cx: &mut Context) -> Self::Compute {
    TreeDerivedDataCompute {
      data: self.data.clone(),
      derive_logic: self.derive_logic,
      payload_source: self.payload_source.describe(cx),
      connectivity_source: self.connectivity_source.describe(cx),
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.data.write().shrink_to_fit();
    self.payload_source.request(request);
    self.connectivity_source.request(request);
  }
}
