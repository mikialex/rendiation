use crate::*;

pub struct ReactiveKVUnion<T1, T2, K, F, O, V1, V2> {
  pub a: T1,
  pub b: T2,
  pub phantom: PhantomData<(K, O, V1, V2)>,
  pub f: F,
}

impl<T1, T2, K, F, O, V1, V2> ReactiveCollection<K, O> for ReactiveKVUnion<T1, T2, K, F, O, V1, V2>
where
  T1: ReactiveCollection<K, V1>,
  T2: ReactiveCollection<K, V2>,
  F: Fn((Option<V1>, Option<V2>)) -> Option<O> + Send + Sync + Copy + 'static,
  K: CKey,
  O: CValue,
  V1: CValue,
  V2: CValue,
{
  type Changes = impl VirtualCollection<K, ValueChange<O>>;
  type View = impl VirtualCollection<K, O>;
  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View) {
    let (t1, a_access) = self.a.poll_changes(cx);
    let (t2, b_access) = self.b.poll_changes(cx);
    let a_access = a_access.into_boxed();
    let b_access = b_access.into_boxed();

    let d = UnionValueChange {
      a: t1.into_boxed(),
      b: t2.into_boxed(),
      f: self.f,
      a_current: a_access.clone(),
      b_current: b_access.clone(),
    };

    let v = UnionCollection {
      a: a_access,
      b: b_access,
      f: self.f,
    };

    (d, v)
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.a.extra_request(request);
    self.b.extra_request(request);
  }
}

#[derive(Clone)]
struct UnionCollection<'a, K, V1, V2, F> {
  a: Box<dyn DynVirtualCollection<K, V1> + 'a>,
  b: Box<dyn DynVirtualCollection<K, V2> + 'a>,
  f: F,
}

impl<'a, K, V1, V2, F, O> VirtualCollection<K, O> for UnionCollection<'a, K, V1, V2, F>
where
  F: Fn((Option<V1>, Option<V2>)) -> Option<O> + Send + Sync + Copy + 'static,
  K: CKey,
  O: CValue,
  V1: CValue,
  V2: CValue,
{
  fn iter_key_value(&self) -> impl Iterator<Item = (K, O)> + '_ {
    let a_side = self
      .a
      .iter_key_value()
      .filter_map(|(k, v1)| (self.f)((Some(v1), self.b.access(&k))).map(|v| (k, v)));

    let b_side = self
      .b
      .iter_key_value()
      .filter(|(k, _)| self.a.access(k).is_none()) // remove the a_side part
      .filter_map(|(k, v2)| (self.f)((self.a.access(&k), Some(v2))).map(|v| (k, v)));

    a_side.chain(b_side)
  }

  fn access(&self, key: &K) -> Option<O> {
    (self.f)((self.a.access(key), self.b.access(key)))
  }
}

#[derive(Clone)]
struct UnionValueChange<'a, K, V1, V2, F> {
  a: Box<dyn DynVirtualCollection<K, ValueChange<V1>> + 'a>,
  b: Box<dyn DynVirtualCollection<K, ValueChange<V2>> + 'a>,
  a_current: Box<dyn DynVirtualCollection<K, V1> + 'a>,
  b_current: Box<dyn DynVirtualCollection<K, V2> + 'a>,
  f: F,
}

impl<'a, K, V1, V2, F, O> VirtualCollection<K, ValueChange<O>>
  for UnionValueChange<'a, K, V1, V2, F>
where
  F: Fn((Option<V1>, Option<V2>)) -> Option<O> + Send + Sync + Copy + 'static,
  K: CKey,
  O: CValue,
  V1: CValue,
  V2: CValue,
{
  fn iter_key_value(&self) -> impl Iterator<Item = (K, ValueChange<O>)> + '_ {
    let checker = make_checker(self.f);

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
        checker(join_change(
          &k,
          &k,
          self.a.access(&k),
          Some(v2),
          &|k| self.a_current.access(k),
          &|k| self.b_current.access(k),
        )?)
        .map(|v| (k, v))
      });

    a_side.chain(b_side)
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
