mod delta;
pub use delta::*;

mod previous_view;
pub use previous_view::*;

mod filter;
pub use filter::*;

mod mutate_target;
pub use mutate_target::*;

mod map;
pub use map::*;

mod union;
pub use union::*;

use crate::*;

#[derive(Clone)]
pub struct DualQuery<T, U> {
  pub view: T,
  pub delta: U,
}

pub type BoxedDynDualQuery<K, V> = DualQuery<BoxedDynQuery<K, V>, BoxedDynQuery<K, ValueChange<V>>>;

pub type MaterializedDeltaDualQuery<Q: DualQueryLike> =
  DualQuery<Q::View, Arc<QueryMaterialized<Q::Key, ValueChange<Q::Value>>>>;

pub trait DualQueryLike: Send + Sync + Clone + 'static {
  type Key: CKey;
  type Value: CValue;
  type Delta: Query<Key = Self::Key, Value = ValueChange<Self::Value>> + 'static;
  type View: Query<Key = Self::Key, Value = Self::Value> + 'static;
  fn view_delta(self) -> (Self::View, Self::Delta);
  fn view_delta_ref(&self) -> (&Self::View, &Self::Delta);

  fn view(self) -> Self::View {
    self.view_delta().0
  }

  fn delta(self) -> Self::Delta {
    self.view_delta().1
  }

  // todo, this is possibly wrong, because filter's iter next has O(n) complexity
  // which is costly to check in main thread.
  // we should add a new is_maybe_empty() method in query trait that allows false positive
  fn is_change_possible_empty(&self) -> bool {
    self.view_delta_ref().1.is_empty()
  }

  /// sometimes materialize delta is necessary, because delta may contains view in some
  /// combinator, and view will be retained, which will cause deadlock in some case
  fn materialize_delta(
    self,
  ) -> DualQuery<Self::View, Arc<QueryMaterialized<Self::Key, ValueChange<Self::Value>>>> {
    let (view, delta) = self.view_delta();
    DualQuery {
      view,
      delta: delta.materialize(),
    }
  }

  fn into_boxed(self) -> BoxedDynDualQuery<Self::Key, Self::Value> {
    let (view, delta) = self.view_delta();
    DualQuery {
      view: view.into_boxed(),
      delta: delta.into_boxed(),
    }
  }

  fn dual_query_map<V2: CValue>(
    self,
    f: impl Fn(Self::Value) -> V2 + Clone + Sync + Send + 'static,
  ) -> impl DualQueryLike<Key = Self::Key, Value = V2> {
    let (view, delta) = self.view_delta();
    DualQuery {
      view: view.map_value(f.clone()),
      delta: delta.delta_map_value(f),
    }
  }

  fn dual_query_map_kv<V2: CValue>(
    self,
    f: impl Fn(&Self::Key, Self::Value) -> V2 + Clone + Sync + Send + 'static,
  ) -> impl DualQueryLike<Key = Self::Key, Value = V2> {
    let (view, delta) = self.view_delta();
    DualQuery { view, delta }.map(f)
  }

  fn dual_query_filter(
    self,
    f: impl Fn(Self::Value) -> bool + Clone + Sync + Send + 'static,
  ) -> impl DualQueryLike<Key = Self::Key, Value = Self::Value> {
    self.dual_query_filter_map(move |v| f(v.clone()).then_some(v))
  }

  fn dual_query_filter_map<V2: CValue>(
    self,
    f: impl Fn(Self::Value) -> Option<V2> + Clone + Sync + Send + 'static,
  ) -> impl DualQueryLike<Key = Self::Key, Value = V2> {
    let (view, delta) = self.view_delta();
    DualQuery {
      view: view.filter_map(f.clone()),
      delta: delta.delta_filter_map(f),
    }
  }

  fn dual_query_select<Q>(
    self,
    other: Q,
    debug_location: Option<&'static Location<'static>>,
  ) -> impl DualQueryLike<Key = Self::Key, Value = Self::Value>
  where
    Q: DualQueryLike<Key = Self::Key, Value = Self::Value>,
  {
    self.dual_query_union(other, move |(a, b)| match (a, b) {
      (Some(_), Some(_)) => {
        unreachable!("key set should not overlap, call site: {debug_location:?}")
      }
      (Some(a), None) => a.into(),
      (None, Some(b)) => b.into(),
      (None, None) => None,
    })
  }

  fn dual_query_zip<Q>(
    self,
    other: Q,
    debug_location: Option<&'static Location<'static>>,
  ) -> impl DualQueryLike<Key = Self::Key, Value = (Self::Value, Q::Value)>
  where
    Q: DualQueryLike<Key = Self::Key>,
  {
    self.dual_query_union(other, move |(a, b)| match (a, b) {
      (Some(a), Some(b)) => Some((a, b)),
      (None, None) => None,
      (None, Some(_)) => unreachable!("zip missing left side, call site: {debug_location:?}"),
      (Some(_), None) => unreachable!("zip missing right side, call site: {debug_location:?}"),
    })
  }

  fn dual_query_filter_by_set<Q>(
    self,
    other: Q,
  ) -> impl DualQueryLike<Key = Self::Key, Value = Self::Value>
  where
    Q: DualQueryLike<Key = Self::Key>,
  {
    self.dual_query_union(other, |(a, b)| match (a, b) {
      (Some(a), Some(_)) => Some(a),
      _ => None,
    })
  }

  fn dual_query_intersect<Q>(
    self,
    other: Q,
  ) -> impl DualQueryLike<Key = Self::Key, Value = (Self::Value, Q::Value)>
  where
    Q: DualQueryLike<Key = Self::Key>,
  {
    self.dual_query_union(other, move |(a, b)| match (a, b) {
      (Some(a), Some(b)) => Some((a, b)),
      _ => None,
    })
  }

  fn dual_query_union<Q, O: CValue>(
    self,
    other: Q,
    f: impl Fn((Option<Self::Value>, Option<Q::Value>)) -> Option<O> + Send + Sync + Copy + 'static,
  ) -> impl DualQueryLike<Key = Self::Key, Value = O>
  where
    Q: DualQueryLike<Key = Self::Key>,
  {
    let (a_access, t1) = self.view_delta();
    let (b_access, t2) = other.view_delta();

    let delta = UnionValueChange {
      a: t1,
      b: t2,
      f,
      a_current: a_access.clone(),
      b_current: b_access.clone(),
    };

    let view = UnionQuery {
      a: a_access,
      b: b_access,
      f,
    };

    DualQuery { view, delta }
  }

  fn fanout<R: TriQueryLike<Value = Self::Key>>(
    self,
    other: R,
  ) -> DualQuery<ChainQuery<R::View, Self::View>, Arc<FastHashMap<R::Key, ValueChange<Self::Value>>>>
  {
    let (getter, upstream_changes) = self.view_delta();
    let (rev_many_view, relation_access, relational_changes) = other.inv_view_view_delta();

    let getter_previous = make_previous(&getter, &upstream_changes);
    let one_acc_previous = make_previous(&relation_access, &relational_changes);

    let mut output = FastHashMap::default();
    {
      let relational_changes = relational_changes.materialize();
      relational_changes
        .iter()
        .for_each(|(k, change)| match change {
          ValueChange::Delta(v, p) => {
            // to get the real previous X, we need the previous o->x mapping
            let p = p.clone().and_then(|p| getter_previous.access(&p));
            if let Some(v) = getter.access(v) {
              output.insert(k.clone(), ValueChange::Delta(v, p));
            } else if let Some(p) = p {
              output.insert(k.clone(), ValueChange::Remove(p));
            }
          }
          ValueChange::Remove(p) => {
            if let Some(p) = getter_previous.access(p) {
              output.insert(k.clone(), ValueChange::Remove(p));
            }
          }
        });
    }
    {
      let upstream_changes = upstream_changes.materialize();
      for (one, delta) in upstream_changes.iter() {
        // the inv_query is the current relation, the previous one's delta is emitted
        // by the above relation change code
        match delta {
          ValueChange::Remove(_p) => rev_many_view.access_multi_visitor(one, &mut |many| {
            if let Some(pre_one) = one_acc_previous.access(&many) {
              if let Some(pre_x) = getter_previous.access(&pre_one) {
                if let Some(ValueChange::Delta(_, _)) = output.get(&many) {
                  // cancel out
                  output.remove(&many);
                } else {
                  output.insert(many.clone(), ValueChange::Remove(pre_x));
                }
              }
            }
          }),
          ValueChange::Delta(change, _p) => {
            rev_many_view.access_multi_visitor(one, &mut |many| {
              if let Some(pre_one) = one_acc_previous.access(&many) {
                let pre_x = getter_previous.access(&pre_one);
                if let Some(ValueChange::Remove(_)) = output.get(&many) {
                  // cancel out
                  output.remove(&many);
                } else {
                  output.insert(many.clone(), ValueChange::Delta(change.clone(), pre_x));
                }
              } else {
                #[allow(clippy::collapsible_else_if)]
                if let Some(ValueChange::Remove(_)) = output.get(&many) {
                  // cancel out
                  output.remove(&many);
                } else {
                  output.insert(many.clone(), ValueChange::Delta(change.clone(), None));
                }
              }
            })
          }
        }
      }
    }

    let d = Arc::new(output);
    let v = relation_access.chain(getter);

    DualQuery { view: v, delta: d }
  }
}

impl<K, V, T, U> DualQueryLike for DualQuery<T, U>
where
  K: CKey,
  V: CValue,
  T: Query<Key = K, Value = V> + Clone + 'static,
  U: Query<Key = K, Value = ValueChange<V>> + Clone + 'static,
{
  type Key = K;
  type Value = V;
  type Delta = U;
  type View = T;

  fn view_delta_ref(&self) -> (&Self::View, &Self::Delta) {
    (&self.view, &self.delta)
  }

  fn view_delta(self) -> (Self::View, Self::Delta) {
    (self.view, self.delta)
  }
}

#[derive(Clone)]
pub struct TriQuery<T, U, V> {
  pub base: DualQuery<T, U>,
  pub rev_many_view: V,
}

pub trait TriQueryLike: DualQueryLike<Value: CKey> {
  type InvView: MultiQuery<Key = Self::Value, Value = Self::Key> + 'static;
  fn inv_view_view_delta(self) -> (Self::InvView, Self::View, Self::Delta);
}

impl<K, V, T, U, Inv> DualQueryLike for TriQuery<T, U, Inv>
where
  K: CKey,
  V: CValue,
  T: Query<Key = K, Value = V> + Clone + 'static,
  U: Query<Key = K, Value = ValueChange<V>> + Clone + 'static,
  Inv: MultiQuery<Key = V, Value = K> + Clone + 'static,
{
  type Key = K;
  type Value = V;
  type Delta = U;
  type View = T;

  fn view_delta_ref(&self) -> (&Self::View, &Self::Delta) {
    (&self.base.view, &self.base.delta)
  }

  fn view_delta(self) -> (Self::View, Self::Delta) {
    (self.base.view, self.base.delta)
  }
}

impl<K, V, T, U, Inv> TriQueryLike for TriQuery<T, U, Inv>
where
  K: CKey,
  V: CKey,
  T: Query<Key = K, Value = V> + Clone + 'static,
  U: Query<Key = K, Value = ValueChange<V>> + Clone + 'static,
  Inv: MultiQuery<Key = V, Value = K> + Clone + 'static,
{
  type InvView = Inv;
  fn inv_view_view_delta(self) -> (Self::InvView, Self::View, Self::Delta) {
    (self.rev_many_view, self.base.view, self.base.delta)
  }
}

pub trait DeltaQueryExt<V>: Query<Value = ValueChange<V>> {
  fn delta_map<V2, F>(self, mapper: F) -> MappedQuery<Self, ValueChangeMapper<F>>
  where
    F: Fn(&Self::Key, V) -> V2 + Sync + Send + Clone + 'static,
    V2: CValue,
  {
    MappedQuery {
      base: self,
      mapper: ValueChangeMapper(mapper),
    }
  }

  fn delta_key_as_value(self) -> impl Query<Key = Self::Key, Value = ValueChange<Self::Key>> {
    self.delta_map(|k, _| k.clone())
  }

  fn delta_map_value<V2, F>(
    self,
    mapper: F,
  ) -> MappedValueQuery<Self, ValueChangeMapperValueOnly<F>>
  where
    F: Fn(V) -> V2 + Sync + Send + Clone + 'static,
    V2: CValue,
  {
    MappedValueQuery {
      base: self,
      mapper: ValueChangeMapperValueOnly(mapper),
    }
  }

  fn delta_filter_map<V2, F>(self, mapper: F) -> FilterMapQueryChange<Self, F>
  where
    F: Fn(V) -> Option<V2> + Sync + Send + Clone + 'static,
    V2: CValue,
  {
    FilterMapQueryChange { base: self, mapper }
  }
}
impl<V, T: Query<Value = ValueChange<V>>> DeltaQueryExt<V> for T {}
