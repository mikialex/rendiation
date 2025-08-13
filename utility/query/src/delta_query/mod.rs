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

use crate::*;

#[derive(Clone)]
pub struct DualQuery<T, U> {
  pub view: T,
  pub delta: U,
}

pub trait DualQueryLike: Send + Sync + Clone + 'static {
  type Key: CKey;
  type Value: CValue;
  type Delta: Query<Key = Self::Key, Value = ValueChange<Self::Value>> + 'static;
  type View: Query<Key = Self::Key, Value = Self::Value> + 'static;
  fn view_delta(self) -> (Self::View, Self::Delta);

  fn view(self) -> Self::View {
    self.view_delta().0
  }

  fn delta(self) -> Self::Delta {
    self.view_delta().1
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

  fn view_delta(self) -> (Self::View, Self::Delta) {
    (self.view, self.delta)
  }
}

#[derive(Clone)]
pub struct TriQuery<T, U, V> {
  pub base: DualQuery<T, U>,
  pub rev_many_view: V,
}

pub trait TriQueryLike: DualQueryLike {
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

  fn view_delta(self) -> (Self::View, Self::Delta) {
    (self.base.view, self.base.delta)
  }
}

impl<K, V, T, U, Inv> TriQueryLike for TriQuery<T, U, Inv>
where
  K: CKey,
  V: CValue,
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

// #[derive(Clone)]
// pub struct SelectDelta<T>(pub T);

// impl<T, V: CValue> Query for SelectDelta<T>
// where
//   T: IteratorProvider + Clone + Send + Sync,
//   T::Item: Query<Value = ValueChange<V>>,
// {
//   type Key = <T::Item as Query>::Key;

//   type Value = ValueChange<V>;

//   fn iter_key_value(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
//     self.0.create_iter().flat_map(|q| {
//       //

//       q.iter_key_value().filter(|(key, delta)|{
//         match delta{
//             ValueChange::Delta(_, _) => true,
//             ValueChange::Remove(_) => if let Some(change) = self.access(key){

//             },
//         }
//       })
//     })
//   }

//   fn access(&self, key: &Self::Key) -> Option<Self::Value> {
//     for q in self.0.create_iter() {
//       if let Some(v) = q.access(key) {
//         return Some(v);
//       }
//     }
//     None
//   }
// }
