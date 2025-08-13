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
  type View = ChainQuery<Relation::View, Upstream::View>;

  #[allow(clippy::collapsible_else_if)]
  fn resolve(&mut self, cx: &QueryResolveCtx) -> (Self::Changes, Self::View) {
    let (relational_changes, relation_access) = self.relations.resolve(cx);
    let (upstream_changes, getter) = self.upstream.resolve(cx);

    let (view, delta) = DualQuery {
      view: getter,
      delta: upstream_changes,
    }
    .fanout(TriQuery {
      base: DualQuery {
        view: relation_access.clone(),
        delta: relational_changes,
      },
      rev_many_view: relation_access,
    })
    .view_delta();

    (delta, view)
  }
}

impl<Upstream, Relation> AsyncQueryCompute for OneToManyFanoutCompute<Upstream, Relation>
where
  Upstream: AsyncQueryCompute<Key = Relation::Value>,
  Relation:
    AsyncQueryCompute<Value: CKey, View: MultiQuery<Key = Relation::Value, Value = Relation::Key>>,
{
  fn create_task(
    &mut self,
    cx: &mut AsyncQueryCtx,
  ) -> QueryComputeTask<(Self::Changes, Self::View)> {
    let upstream = self.upstream.create_task(cx);
    let relations = self.relations.create_task(cx);
    let parents = futures::future::join(upstream, relations);
    cx.then_spawn_compute(parents, |(upstream, relations)| OneToManyFanoutCompute {
      upstream,
      relations,
    })
    .into_boxed_future()
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
