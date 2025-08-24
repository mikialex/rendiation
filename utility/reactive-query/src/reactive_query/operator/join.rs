use crate::*;

impl<A, B> ReactiveQuery for CrossJoinQuery<A, B>
where
  A: ReactiveQuery,
  B: ReactiveQuery,
{
  type Key = (A::Key, B::Key);
  type Value = (A::Value, B::Value);
  type Compute = CrossJoinQuery<A::Compute, B::Compute>;
  fn describe(&self, cx: &mut Context) -> Self::Compute {
    CrossJoinQuery {
      a: self.a.describe(cx),
      b: self.b.describe(cx),
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.a.request(request);
    self.b.request(request);
  }
}

impl<A, B> QueryCompute for CrossJoinQuery<A, B>
where
  A: QueryCompute,
  B: QueryCompute,
{
  type Key = (A::Key, B::Key);
  type Value = (A::Value, B::Value);
  type Changes = CrossJoinValueChange<A::View, B::View, A::Changes, B::Changes>;
  type View = CrossJoinQuery<A::View, B::View>;

  fn resolve(&mut self, cx: &QueryResolveCtx) -> (Self::Changes, Self::View) {
    let (t1, a_access) = self.a.resolve(cx);
    let (t2, b_access) = self.b.resolve(cx);

    let d = CrossJoinValueChange {
      a: t1,
      b: t2,
      a_current: a_access.clone(),
      b_current: b_access.clone(),
    };

    let v = CrossJoinQuery {
      a: a_access,
      b: b_access,
    };

    (d, v)
  }
}

impl<A, B> AsyncQueryCompute for CrossJoinQuery<A, B>
where
  A: AsyncQueryCompute,
  B: AsyncQueryCompute,
{
  fn create_task(
    &mut self,
    cx: &mut AsyncQueryCtx,
  ) -> QueryComputeTask<(Self::Changes, Self::View)> {
    let a = self.a.create_task(cx);
    let b = self.b.create_task(cx);
    let c = cx.resolve_cx().clone();
    futures::future::join(a, b)
      .map(move |(a, b)| CrossJoinQuery { a, b }.resolve(&c))
      .into_boxed_future()
  }
}

#[derive(Clone)]
pub struct CrossJoinValueChange<A, B, DA, DB> {
  pub a: DA,
  pub b: DB,
  pub a_current: A,
  pub b_current: B,
}

impl<A, B, DA, DB> Query for CrossJoinValueChange<A, B, DA, DB>
where
  DA: Query<Key = A::Key, Value = ValueChange<A::Value>>,
  DB: Query<Key = B::Key, Value = ValueChange<B::Value>>,
  A: Query,
  B: Query,
{
  type Key = (A::Key, B::Key);
  type Value = ValueChange<(A::Value, B::Value)>;
  fn iter_key_value(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    let cross_section = self.a.iter_key_value().flat_map(move |(k1, v1_change)| {
      self.b.iter_key_value().map(move |(k2, v2_change)| {
        join_change(
          &k1,
          &k2,
          Some(v1_change.clone()),
          Some(v2_change),
          &|k| self.a_current.access(k),
          &|k| self.b_current.access(k),
        )
        .map(|v| ((k1.clone(), k2), exist_both(v).unwrap()))
        .unwrap()
      })
    });

    let a_side_change_with_b = self.a.iter_key_value().flat_map(move |(k1, v1_change)| {
      self
        .b_current
        .iter_key_value()
        .filter(move |(k2, _)| !self.b.contains(k2))
        .map(move |(k2, _)| {
          join_change(
            &k1,
            &k2,
            Some(v1_change.clone()),
            None,
            &|k| self.a_current.access(k),
            &|k| self.b_current.access(k),
          )
          .map(|v| ((k1.clone(), k2), exist_both(v).unwrap()))
          .unwrap()
        })
    });

    let a_side_change_with_b = avoid_huge_debug_symbols_by_boxing_iter(a_side_change_with_b);

    let b_side_change_with_a = self.b.iter_key_value().flat_map(move |(k2, v2_change)| {
      self
        .a_current
        .iter_key_value()
        .filter(move |(k1, _)| !self.a.contains(k1))
        .map(move |(k1, _)| {
          join_change(
            &k1,
            &k2,
            None,
            Some(v2_change.clone()),
            &|k| self.a_current.access(k),
            &|k| self.b_current.access(k),
          )
          .map(|v| ((k1, k2.clone()), exist_both(v).unwrap()))
          .unwrap()
        })
    });

    let b_side_change_with_a = avoid_huge_debug_symbols_by_boxing_iter(b_side_change_with_a);

    cross_section
      .chain(a_side_change_with_b)
      .chain(b_side_change_with_a)
  }

  fn access(&self, (k1, k2): &(A::Key, B::Key)) -> Option<Self::Value> {
    join_change(
      &k1,
      &k2,
      self.a.access(k1),
      self.b.access(k2),
      &|k| self.a_current.access(k),
      &|k| self.b_current.access(k),
    )
    .and_then(exist_both)
  }
}

fn exist_both<V1, V2>(
  change: ValueChange<(Option<V1>, Option<V2>)>,
) -> Option<ValueChange<(V1, V2)>> {
  match change {
    ValueChange::Delta(new, previous) => Some(ValueChange::Delta(
      new.0.zip(new.1)?,
      previous.and_then(|v| v.0.zip(v.1)),
    )),
    ValueChange::Remove((v1, v2)) => Some(ValueChange::Remove(v1.zip(v2)?)),
  }
}
