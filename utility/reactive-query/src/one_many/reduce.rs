use crate::*;

pub struct ManyToOneReduce<Upstream, Relation, V> {
  pub upstream: Upstream,
  pub relations: Relation,
  pub ref_count: Arc<RwLock<FastHashMap<V, u32>>>,
}

impl<Upstream, Relation> ReactiveQuery for ManyToOneReduce<Upstream, Relation, Relation::Value>
where
  Upstream: ReactiveQuery<Value = ()>,
  Relation: ReactiveQuery<Key = Upstream::Key>,
  Relation::Value: CKey,
{
  type Key = Relation::Value;
  type Value = ();
  type Compute = ManyToOneReduce<Upstream::Compute, Relation::Compute, Relation::Value>;

  fn describe(&self, cx: &mut Context) -> Self::Compute {
    ManyToOneReduce {
      upstream: self.upstream.describe(cx),
      relations: self.relations.describe(cx),
      ref_count: self.ref_count.clone(),
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

impl<Upstream, Relation> QueryCompute for ManyToOneReduce<Upstream, Relation, Relation::Value>
where
  Upstream: QueryCompute<Value = ()>,
  Relation: QueryCompute<Key = Upstream::Key>,
  Relation::Value: CKey,
{
  type Key = Relation::Value;
  type Value = ();

  type Changes = Arc<FastHashMap<Self::Key, ValueChange<()>>>;
  type View = ManyToOneReduceCurrentView<Self::Key>;

  fn resolve(&mut self, cx: &QueryResolveCtx) -> (Self::Changes, Self::View) {
    let (relational_changes, one_acc) = self.relations.resolve(cx);
    let (upstream_changes, getter) = self.upstream.resolve(cx);

    let getter_previous = make_previous(&getter, &upstream_changes);

    let mut output = FastHashMap::default();
    let mut ref_counts = self.ref_count.write();

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

    drop(ref_counts);

    let d = Arc::new(output);
    let v = ManyToOneReduceCurrentView {
      ref_count: self.ref_count.make_read_holder(),
    };

    (d, v)
  }
}

impl<Upstream, Relation> AsyncQueryCompute for ManyToOneReduce<Upstream, Relation, Relation::Value>
where
  Upstream: AsyncQueryCompute<Value = ()>,
  Relation: AsyncQueryCompute<Key = Upstream::Key>,
  Relation::Value: CKey,
{
  fn create_task(
    &mut self,
    cx: &mut AsyncQueryCtx,
  ) -> QueryComputeTask<(Self::Changes, Self::View)> {
    let upstream = self.upstream.create_task(cx);
    let relations = self.relations.create_task(cx);
    let ref_count = self.ref_count.clone();

    let parents = futures::future::join(upstream, relations);
    cx.then_spawn_compute(parents, |(upstream, relations)| ManyToOneReduce {
      upstream,
      relations,
      ref_count,
    })
    .into_boxed_future()
  }
}

#[derive(Clone)]
pub struct ManyToOneReduceCurrentView<O: CKey> {
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
