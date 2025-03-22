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

  type Changes = impl Query<Key = Self::Key, Value = ValueChange<Self::Value>> + 'static;
  type View = impl Query<Key = Self::Key, Value = Self::Value> + 'static;

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
      phantom: PhantomData,
    };

    (d, v)
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
struct OneToManyFanoutCurrentView<U, R, O> {
  upstream: U,
  relation: R,
  phantom: PhantomData<O>,
}

impl<U, R, O, M, X> Query for OneToManyFanoutCurrentView<U, R, O>
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

pub struct ManyToOneReduce<Upstream, Relation>
where
  Relation: ReactiveQuery<Value: CKey>,
{
  pub upstream: Upstream,
  pub relations: Relation,
  pub ref_count: Arc<RwLock<FastHashMap<Relation::Value, u32>>>,
}

pub struct ManyToOneReduceCompute<Upstream, Relation>
where
  Relation: QueryCompute<Value: CKey>,
{
  pub upstream: Upstream,
  pub relations: Relation,
  pub ref_count: Option<LockWriteGuardHolder<FastHashMap<Relation::Value, u32>>>,
}

impl<Upstream, Relation> ReactiveQuery for ManyToOneReduce<Upstream, Relation>
where
  Upstream: ReactiveQuery<Value = ()>,
  Relation: ReactiveQuery<Key = Upstream::Key>,
  Relation::Value: CKey,
{
  type Key = Relation::Value;
  type Value = ();
  type Compute = ManyToOneReduceCompute<Upstream::Compute, Relation::Compute>;

  fn describe(&self, cx: &mut Context) -> Self::Compute {
    ManyToOneReduceCompute {
      upstream: self.upstream.describe(cx),
      relations: self.relations.describe(cx),
      ref_count: Some(self.ref_count.make_write_holder()),
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.upstream.request(request);
    self.relations.request(request);
    match request {
      ReactiveQueryRequest::MemoryShrinkToFit => {
        self.ref_count.write().shrink_to_fit();
      }
    }
  }
}

impl<Upstream, Relation> QueryCompute for ManyToOneReduceCompute<Upstream, Relation>
where
  Upstream: QueryCompute<Value = ()>,
  Relation: QueryCompute<Key = Upstream::Key>,
  Relation::Value: CKey,
{
  type Key = Relation::Value;
  type Value = ();

  type Changes = impl Query<Key = Self::Key, Value = ValueChange<Self::Value>> + 'static;
  type View = impl Query<Key = Self::Key, Value = Self::Value> + 'static;

  fn resolve(&mut self) -> (Self::Changes, Self::View) {
    let (relational_changes, one_acc) = self.relations.resolve();
    let (upstream_changes, getter) = self.upstream.resolve();

    let getter_previous = make_previous(&getter, &upstream_changes);

    let mut output = FastHashMap::default();
    let mut ref_counts = self.ref_count.take().unwrap();

    {
      let relational_changes = relational_changes.materialize();
      for (key, change) in relational_changes.iter_key_value() {
        let old_value = change.old_value();
        let new_value = change.new_value();

        if let Some(ov) = old_value {
          if getter_previous.access(&key).is_some() {
            let ref_count = ref_counts.get_mut(ov).unwrap();
            *ref_count -= 1;
            if *ref_count == 0 {
              ref_counts.remove(ov);
              output.insert(ov.clone(), ValueChange::Remove(()));
            }
          }
        }

        if let Some(nv) = new_value {
          if getter_previous.access(&key).is_some() {
            let ref_count = ref_counts.entry(nv.clone()).or_insert_with(|| {
              if let Some(ValueChange::Remove(_)) = output.get(nv) {
                // cancel out
                output.remove(nv);
              } else {
                output.insert(nv.clone(), ValueChange::Delta((), None));
              }
              0
            });
            *ref_count += 1;
          }
        }
      }
    }

    {
      let upstream_changes = upstream_changes.materialize();
      for (many, delta) in upstream_changes.iter_key_value() {
        match delta {
          ValueChange::Remove(_) => {
            // we should remove from the new old relation
            if let Some(one) = one_acc.access(&many) {
              if let Some(ref_count) = ref_counts.get_mut(&one) {
                *ref_count -= 1;
                if *ref_count == 0 {
                  ref_counts.remove(&one);

                  if let Some(ValueChange::Delta(_, _)) = output.get(&one) {
                    // cancel out
                    output.remove(&one);
                  } else {
                    output.insert(one.clone(), ValueChange::Remove(()));
                  }
                }
              }
            }
          }
          ValueChange::Delta(_, p) => {
            if p.is_none() {
              // should check if it is insert
              // we should insert into the new directed relation
              if let Some(one) = one_acc.access(&many) {
                let ref_count = ref_counts.entry(one.clone()).or_insert_with(|| {
                  if let Some(ValueChange::Remove(_)) = output.get(&one) {
                    // cancel out
                    output.remove(&one);
                  } else {
                    output.insert(one.clone(), ValueChange::Delta((), None));
                  }
                  0
                });
                *ref_count += 1;
              }
            }
          }
        }
      }
    }

    let d = Arc::new(output);
    let v = ManyToOneReduceCurrentView {
      ref_count: ref_counts.downgrade_to_read(),
    };

    (d, v)
  }
}

#[derive(Clone)]
struct ManyToOneReduceCurrentView<O: CKey> {
  ref_count: LockReadGuardHolder<FastHashMap<O, u32>>,
}

impl<O: CKey> Query for ManyToOneReduceCurrentView<O> {
  type Key = O;
  type Value = ();
  fn iter_key_value(&self) -> impl Iterator<Item = (O, ())> + '_ {
    self.ref_count.iter().map(|(k, _)| (k.clone(), ()))
  }

  fn access(&self, key: &O) -> Option<()> {
    self.ref_count.contains_key(key).then_some(())
  }
}
