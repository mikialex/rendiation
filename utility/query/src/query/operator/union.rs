use crate::*;

#[derive(Clone)]
pub struct SelectMany<T>(pub T);

impl<T> Query for SelectMany<T>
where
  T: IteratorProvider + Clone + Send + Sync,
  T::Item: Query,
{
  type Key = <T::Item as Query>::Key;

  type Value = <T::Item as Query>::Value;

  fn iter_key_value(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    self.0.create_iter().flat_map(|q| q.iter_key_value())
  }

  fn access(&self, key: &Self::Key) -> Option<Self::Value> {
    for q in self.0.create_iter() {
      if let Some(v) = q.access(key) {
        return Some(v);
      }
    }
    None
  }

  fn has_item_hint(&self) -> bool {
    self.0.create_iter().any(|q| q.has_item_hint())
  }
}

#[test]
fn test_select_query() {
  let mut q1 = FastHashMap::default();
  q1.insert(1u32, "a".to_string());

  let mut q2 = FastHashMap::default();
  q2.insert(2u32, "b".to_string());

  let mut q3 = FastHashMap::default();
  q3.insert(3u32, "c".to_string());

  let queries = vec![q1, q2, q3];
  let select = SelectMany(queries);

  super::validate_query_consistency(&select);
  assert_eq!(select.access(&1), Some("a".to_string()));
  assert_eq!(select.access(&2), Some("b".to_string()));
  assert_eq!(select.access(&3), Some("c".to_string()));
  assert_eq!(select.access(&4), None);
}

#[test]
fn test_select_empty_query() {
  let queries: Vec<FastHashMap<u32, String>> = vec![];
  let select = SelectMany(queries);

  super::validate_query_consistency(&select);
  assert_eq!(select.access(&1), None);
}

#[derive(Clone)]
pub struct UnionQuery<A, B, F> {
  pub a: A,
  pub b: B,
  pub f: F,
}

impl<A, B, F, O> Query for UnionQuery<A, B, F>
where
  A: Query,
  B: Query<Key = A::Key>,
  F: Fn((Option<A::Value>, Option<B::Value>)) -> Option<O> + Send + Sync + Clone + 'static,

  O: CValue,
{
  type Key = A::Key;
  type Value = O;
  fn iter_key_value(&self) -> impl Iterator<Item = (A::Key, O)> + '_ {
    let a_side = self
      .a
      .iter_key_value()
      .filter_map(|(k, v1)| (self.f)((Some(v1), self.b.access(&k))).map(|v| (k, v)));

    let b_side = self
      .b
      .iter_key_value()
      .filter(|(k, _)| self.a.access(k).is_none()) // remove the a_side part
      .filter_map(|(k, v2)| (self.f)((self.a.access(&k), Some(v2))).map(|v| (k, v)));

    avoid_huge_debug_symbols_by_boxing_iter(a_side.chain(b_side))
  }

  fn access(&self, key: &A::Key) -> Option<O> {
    (self.f)((self.a.access(key), self.b.access(key)))
  }

  fn has_item_hint(&self) -> bool {
    self.a.has_item_hint() || self.b.has_item_hint()
  }
}

#[test]
fn test_union_query() {
  // a covers {1, 2}, b covers {2, 3} — key 2 overlaps
  let mut a = FastHashMap::default();
  a.insert(1u32, 10i32);
  a.insert(2, 20);

  let mut b = FastHashMap::default();
  b.insert(2u32, 30);
  b.insert(3, 40);

  let unioned = UnionQuery {
    a,
    b,
    f: |(va, vb): (Option<i32>, Option<i32>)| match (va, vb) {
      (Some(a), Some(b)) => Some(a + b),
      (Some(a), None) => Some(a),
      (None, Some(b)) => Some(b),
      (None, None) => None,
    },
  };

  super::validate_query_consistency(&unioned);
  assert_eq!(unioned.access(&1), Some(10));
  assert_eq!(unioned.access(&2), Some(50));
  assert_eq!(unioned.access(&3), Some(40));
  assert_eq!(unioned.access(&4), None);
}

#[test]
fn test_union_query_left_only() {
  let mut a = FastHashMap::default();
  a.insert(1u32, "left".to_string());

  let b: FastHashMap<u32, String> = FastHashMap::default();

  let unioned = UnionQuery {
    a,
    b,
    f: |(va, vb): (Option<String>, Option<String>)| match (va, vb) {
      (Some(a), None) => Some(a),
      (None, Some(b)) => Some(b),
      _ => None,
    },
  };

  super::validate_query_consistency(&unioned);
  assert_eq!(unioned.access(&1), Some("left".to_string()));
  assert_eq!(unioned.access(&2), None);
}
