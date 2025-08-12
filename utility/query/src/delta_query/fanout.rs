use crate::*;

impl<T, U> DualQuery<T, U> {
  pub fn compute_fanout<KMany, KOne, V, X, Y, Z>(
    self,
    relation: TriQuery<X, Y, Z>,
  ) -> DualQuery<ChainQuery<X, T>, Arc<FastHashMap<KMany, ValueChange<V>>>>
  where
    KMany: CKey,
    KOne: CKey,
    V: CValue,
    T: Query<Key = KOne, Value = V>,
    U: Query<Key = KOne, Value = ValueChange<V>>,
    X: Query<Key = KMany, Value = KOne>,
    Y: Query<Key = KMany, Value = ValueChange<KOne>>,
    Z: MultiQuery<Key = KOne, Value = KMany>,
  {
    let DualQuery {
      view: getter,
      delta: upstream_changes,
    } = self;

    let TriQuery {
      base: DualQuery {
        view: relation_access,
        delta: relational_changes,
      },
      rev_many_view,
    } = relation;

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
