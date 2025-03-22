use crate::*;

impl<T1, T2, F, O> ReactiveQuery for UnionQuery<T1, T2, F>
where
  T1: ReactiveQuery,
  T2: ReactiveQuery<Key = T1::Key>,
  F: Fn((Option<T1::Value>, Option<T2::Value>)) -> Option<O> + Send + Sync + Copy + 'static,
  O: CValue,
{
  type Key = T1::Key;
  type Value = O;
  type Compute = impl QueryCompute<Key = Self::Key, Value = Self::Value>;

  fn describe(&self, cx: &mut Context) -> Self::Compute {
    UnionQuery {
      a: self.a.describe(cx),
      b: self.b.describe(cx),
      f: self.f,
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.a.request(request);
    self.b.request(request);
  }
}

impl<T1, T2, F, O> QueryCompute for UnionQuery<T1, T2, F>
where
  T1: QueryCompute,
  T2: QueryCompute<Key = T1::Key>,
  F: Fn((Option<T1::Value>, Option<T2::Value>)) -> Option<O> + Send + Sync + Copy + 'static,
  O: CValue,
{
  type Key = T1::Key;
  type Value = O;
  type Changes = UnionValueChange<T1::View, T2::View, T1::Changes, T2::Changes, F>;
  type View = UnionQuery<T1::View, T2::View, F>;

  fn resolve(&mut self) -> (Self::Changes, Self::View) {
    let (t1, a_access) = self.a.resolve();
    let (t2, b_access) = self.b.resolve();
    let a_access = a_access;
    let b_access = b_access;

    let d = UnionValueChange {
      a: t1,
      b: t2,
      f: self.f,
      a_current: a_access.clone(),
      b_current: b_access.clone(),
    };

    let v = UnionQuery {
      a: a_access,
      b: b_access,
      f: self.f,
    };

    (d, v)
  }
}

#[derive(Clone)]
pub struct UnionValueChange<A, B, AD, BD, F> {
  pub a: AD,
  pub b: BD,
  pub a_current: A,
  pub b_current: B,
  pub f: F,
}

impl<A, B, AD, BD, K, V1, V2, F, O> Query for UnionValueChange<A, B, AD, BD, F>
where
  A: Query<Key = K, Value = V1>,
  B: Query<Key = K, Value = V2>,
  AD: Query<Key = K, Value = ValueChange<V1>>,
  BD: Query<Key = K, Value = ValueChange<V2>>,
  F: Fn((Option<V1>, Option<V2>)) -> Option<O> + Send + Sync + Copy + 'static,
  K: CKey,
  O: CValue,
  V1: CValue,
  V2: CValue,
{
  type Key = K;
  type Value = ValueChange<O>;
  fn iter_key_value(&self) -> impl Iterator<Item = (K, ValueChange<O>)> + '_ {
    let checker = make_checker(self.f);
    let checker2 = checker.clone();

    let a_side = self.a.iter_key_value().filter_map(move |(k, v1)| {
      checker(join_change(
        &k,
        &k,
        Some(v1),
        self.b.access(&k),
        &|k| self.a_current.access(k),
        &|k| self.b_current.access(k),
      )?)
      .map(|v| (k, v))
    });

    let b_side = self
      .b
      .iter_key_value()
      .filter(|(k, _)| self.a.access(k).is_none()) // remove the a_side part
      .filter_map(move |(k, v2)| {
        checker2(join_change(
          &k,
          &k,
          self.a.access(&k),
          Some(v2),
          &|k| self.a_current.access(k),
          &|k| self.b_current.access(k),
        )?)
        .map(|v| (k, v))
      });

    avoid_huge_debug_symbols_by_boxing_iter(a_side.chain(b_side))
  }

  fn access(&self, key: &K) -> Option<ValueChange<O>> {
    let checker = make_checker(self.f);

    checker(join_change(
      key,
      key,
      self.a.access(key),
      self.b.access(key),
      &|k| self.a_current.access(k),
      &|k| self.b_current.access(k),
    )?)
  }
}
