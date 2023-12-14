use crate::*;

pub struct ReactiveKVUnion<T1, T2, K, F, O, V1, V2> {
  pub a: BufferedCollection<T1, K, V1>,
  pub b: BufferedCollection<T2, K, V2>,
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
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<K, O> {
    let waker = cx.waker().clone();
    let (t1, t2) = rayon::join(
      || {
        let mut cx = Context::from_waker(&waker);
        self.a.poll_changes(&mut cx)
      },
      || {
        let mut cx = Context::from_waker(&waker);
        self.b.poll_changes(&mut cx)
      },
    );

    let a_access = self.a.access();
    let b_access = self.b.access();

    if a_access.is_blocked() || b_access.is_blocked() || t1.is_blocked() || t2.is_blocked() {
      drop(a_access);
      drop(b_access);
      if let CPoll::Ready(Poll::Ready(v)) = t1 {
        self.a.put_back_to_buffered(v.materialize());
      }
      if let CPoll::Ready(Poll::Ready(v)) = t2 {
        self.b.put_back_to_buffered(v.materialize());
      }
      return CPoll::Blocked;
    };

    let a_access = a_access.unwrap();
    let b_access = b_access.unwrap();
    let t1 = t1.unwrap();
    let t2 = t2.unwrap();

    CPoll::Ready(Poll::Ready(Box::new(UnionCollectionDelta {
      a: match t1 {
        Poll::Ready(delta) => delta,
        Poll::Pending => Box::new(()),
      },
      b: match t2 {
        Poll::Ready(delta) => delta,
        Poll::Pending => Box::new(()),
      },
      f: self.f,
      a_current: a_access,
      b_current: b_access,
    })))
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.a.extra_request(request);
    self.b.extra_request(request);
  }

  fn access(&self) -> PollCollectionCurrent<K, O> {
    let access_a = match self.a.access() {
      CPoll::Ready(v) => v,
      CPoll::Blocked => return CPoll::Blocked,
    };
    let access_b = match self.b.access() {
      CPoll::Ready(v) => v,
      CPoll::Blocked => return CPoll::Blocked,
    };
    CPoll::Ready(Box::new(UnionCollection {
      a: access_a,
      b: access_b,
      f: self.f,
    }))
  }
}

#[derive(Clone)]
struct UnionCollection<'a, K, V1, V2, F> {
  a: Box<dyn VirtualCollection<K, V1> + 'a>,
  b: Box<dyn VirtualCollection<K, V2> + 'a>,
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
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (K, O)> + '_> {
    let a_side = self
      .a
      .iter_key_value()
      .filter_map(|(k, v1)| (self.f)((Some(v1), self.b.access(&k))).map(|v| (k, v)));

    let b_side = self
      .b
      .iter_key_value()
      .filter(|(k, _)| self.a.access(k).is_none()) // remove the a_side part
      .filter_map(|(k, v2)| (self.f)((self.a.access(&k), Some(v2))).map(|v| (k, v)));

    Box::new(a_side.chain(b_side))
  }

  fn access(&self, key: &K) -> Option<O> {
    (self.f)((self.a.access(key), self.b.access(key)))
  }
}

#[derive(Clone)]
struct UnionCollectionDelta<'a, K, V1, V2, F> {
  a: Box<dyn VirtualCollection<K, CollectionDelta<K, V1>> + 'a>,
  b: Box<dyn VirtualCollection<K, CollectionDelta<K, V2>> + 'a>,
  a_current: Box<dyn VirtualCollection<K, V1> + 'a>,
  b_current: Box<dyn VirtualCollection<K, V2> + 'a>,
  f: F,
}

impl<'a, K, V1, V2, F, O> VirtualCollection<K, CollectionDelta<K, O>>
  for UnionCollectionDelta<'a, K, V1, V2, F>
where
  F: Fn((Option<V1>, Option<V2>)) -> Option<O> + Send + Sync + Copy + 'static,
  K: CKey,
  O: CValue,
  V1: CValue,
  V2: CValue,
{
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (K, CollectionDelta<K, O>)> + '_> {
    let checker = make_checker(self.f);

    let a_side = self.a.iter_key_value().filter_map(move |(k, v1)| {
      checker(union(
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
        checker(union(
          self.a.access(&k),
          Some(v2),
          &|k| self.a_current.access(k),
          &|k| self.b_current.access(k),
        )?)
        .map(|v| (k, v))
      });

    Box::new(a_side.chain(b_side))
  }

  fn access(&self, key: &K) -> Option<CollectionDelta<K, O>> {
    let checker = make_checker(self.f);

    checker(union(
      self.a.access(key),
      self.b.access(key),
      &|k| self.a_current.access(k),
      &|k| self.b_current.access(k),
    )?)
  }
}

fn union<K: Clone, V1: Clone, V2: Clone>(
  change1: Option<CollectionDelta<K, V1>>,
  change2: Option<CollectionDelta<K, V2>>,
  v1_current: &impl Fn(&K) -> Option<V1>,
  v2_current: &impl Fn(&K) -> Option<V2>,
) -> Option<CollectionDelta<K, (Option<V1>, Option<V2>)>> {
  let r = match (change1, change2) {
    (None, None) => return None,
    (None, Some(change2)) => match change2 {
      CollectionDelta::Delta(k, v2, p2) => {
        let v1_current = v1_current(&k);
        CollectionDelta::Delta(k, (v1_current.clone(), Some(v2)), Some((v1_current, p2)))
      }
      CollectionDelta::Remove(k, p2) => {
        if let Some(v1_current) = v1_current(&k) {
          CollectionDelta::Delta(
            k,
            (Some(v1_current.clone()), None),
            Some((Some(v1_current), Some(p2))),
          )
        } else {
          CollectionDelta::Remove(k, (None, Some(p2)))
        }
      }
    },
    (Some(change1), None) => match change1 {
      CollectionDelta::Delta(k, v1, p1) => {
        let v2_current = v2_current(&k);
        CollectionDelta::Delta(k, (Some(v1), v2_current.clone()), Some((p1, v2_current)))
      }
      CollectionDelta::Remove(k, p1) => {
        if let Some(v2_current) = v2_current(&k) {
          CollectionDelta::Delta(
            k,
            (None, Some(v2_current.clone())),
            Some((Some(p1), Some(v2_current))),
          )
        } else {
          CollectionDelta::Remove(k, (Some(p1), None))
        }
      }
    },
    (Some(change1), Some(change2)) => match (change1, change2) {
      (CollectionDelta::Delta(k, v1, p1), CollectionDelta::Delta(_, v2, p2)) => {
        CollectionDelta::Delta(k, (Some(v1), Some(v2)), Some((p1, p2)))
      }
      (CollectionDelta::Delta(_, v1, p1), CollectionDelta::Remove(k, p2)) => {
        CollectionDelta::Delta(k.clone(), (Some(v1), v2_current(&k)), Some((p1, Some(p2))))
      }
      (CollectionDelta::Remove(k, p1), CollectionDelta::Delta(_, v2, p2)) => {
        CollectionDelta::Delta(k.clone(), (v1_current(&k), Some(v2)), Some((Some(p1), p2)))
      }
      (CollectionDelta::Remove(k, p1), CollectionDelta::Remove(_, p2)) => {
        CollectionDelta::Remove(k, (Some(p1), Some(p2)))
      }
    },
  };

  if let CollectionDelta::Delta(k, new, Some((None, None))) = r {
    return CollectionDelta::Delta(k, new, None).into();
  }

  r.into()
}
