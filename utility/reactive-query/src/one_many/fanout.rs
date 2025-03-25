use crate::*;

pub struct OneToManyFanout<Upstream, Relation>
where
  Upstream: ReactiveQuery,
  Relation: ReactiveOneToManyRelation,
{
  pub upstream: Upstream,
  pub relations: Relation,
}

pub struct OneToManyFanoutCompute<Upstream, Relation>
where
  Upstream: QueryCompute,
  Relation: QueryCompute<Value: CKey>,
{
  pub upstream: Upstream,
  pub relations: Relation,
}

impl<Upstream, Relation> QueryCompute for OneToManyFanoutCompute<Upstream, Relation>
where
  Upstream: QueryCompute<Key = Relation::Value>,
  Relation:
    QueryCompute<Value: CKey, View: MultiQuery<Key = Relation::Value, Value = Relation::Key>>,
{
  type Key = Relation::Key;
  type Value = Upstream::Value;

  type Changes = Arc<FastHashMap<Self::Key, ValueChange<Self::Value>>>;
  type View = OneToManyFanoutCurrentView<Upstream::View, Relation::View>;

  #[allow(clippy::collapsible_else_if)]
  fn resolve(&mut self) -> (Self::Changes, Self::View) {
    let (relational_changes, relation_access) = self.relations.resolve();
    let (upstream_changes, getter) = self.upstream.resolve();

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
          ValueChange::Remove(_p) => relation_access.access_multi_visitor(one, &mut |many| {
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
            relation_access.access_multi_visitor(one, &mut |many| {
              if let Some(pre_one) = one_acc_previous.access(&many) {
                let pre_x = getter_previous.access(&pre_one);
                if let Some(ValueChange::Remove(_)) = output.get(&many) {
                  // cancel out
                  output.remove(&many);
                } else {
                  output.insert(many.clone(), ValueChange::Delta(change.clone(), pre_x));
                }
              } else {
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
    let v = OneToManyFanoutCurrentView {
      upstream: getter,
      relation: relation_access,
    };

    (d, v)
  }
}

impl<Upstream, Relation> AsyncQueryCompute for OneToManyFanoutCompute<Upstream, Relation>
where
  Upstream: AsyncQueryCompute<Key = Relation::Value>,
  Relation:
    AsyncQueryCompute<Value: CKey, View: MultiQuery<Key = Relation::Value, Value = Relation::Key>>,
{
  type Task = impl Future<Output = (Self::Changes, Self::View)>;

  fn create_task(&mut self, cx: &mut AsyncQueryCtx) -> Self::Task {
    let upstream = self.upstream.create_task(cx);
    let relations = self.relations.create_task(cx);
    let parents = futures::future::join(upstream, relations);
    cx.then_spawn(parents, |(upstream, relations)| {
      OneToManyFanoutCompute {
        upstream,
        relations,
      }
      .resolve()
    })
  }
}

impl<Upstream, Relation> ReactiveQuery for OneToManyFanout<Upstream, Relation>
where
  Upstream: ReactiveQuery<Key = Relation::One>,
  Relation: ReactiveOneToManyRelation + 'static,
{
  type Key = Relation::Many;
  type Value = Upstream::Value;

  type Compute = OneToManyFanoutCompute<Upstream::Compute, Relation::Compute>;

  fn describe(&self, cx: &mut Context) -> Self::Compute {
    OneToManyFanoutCompute {
      upstream: self.upstream.describe(cx),
      relations: self.relations.describe(cx),
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.upstream.request(request);
    self.relations.request(request);
  }
}

#[derive(Clone)]
pub struct OneToManyFanoutCurrentView<U, R> {
  upstream: U,
  relation: R,
}

impl<U, R, O, M, X> Query for OneToManyFanoutCurrentView<U, R>
where
  O: CKey,
  M: CKey,
  X: CValue,
  U: Query<Key = O, Value = X>,
  R: Query<Key = M, Value = O>,
{
  type Key = M;
  type Value = X;
  fn iter_key_value(&self) -> impl Iterator<Item = (M, X)> + '_ {
    // this is pretty costly
    self
      .relation
      .iter_key_value()
      .filter_map(|(k, _v)| self.access(&k).map(|v| (k, v)))
  }

  fn access(&self, key: &M) -> Option<X> {
    let o = self.relation.access(key)?;
    self.upstream.access(&o)
  }
}
