use crate::*;

#[derive(Clone)]
pub struct CrossJoinQuery<A, B> {
  pub a: A,
  pub b: B,
}

impl<A, B> Query for CrossJoinQuery<A, B>
where
  A: Query,
  B: Query,
{
  type Key = (A::Key, B::Key);
  type Value = (A::Value, B::Value);
  fn iter_key_value(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    self.a.iter_key_value().flat_map(move |(k1, v1)| {
      self
        .b
        .iter_key_value()
        .map(move |(k2, v2)| ((k1.clone(), k2), (v1.clone(), v2)))
    })
  }

  fn access(&self, key: &Self::Key) -> Option<Self::Value> {
    self.a.access(&key.0).zip(self.b.access(&key.1))
  }

  fn has_item_hint(&self) -> bool {
    self.a.has_item_hint() || self.b.has_item_hint()
  }
}

#[test]
fn test_cross_join_query() {
  let mut a = FastHashMap::default();
  a.insert(1u32, "a".to_string());
  a.insert(2, "b".to_string());

  let mut b = FastHashMap::default();
  b.insert(10u32, "x".to_string());
  b.insert(20, "y".to_string());

  let joined = CrossJoinQuery { a, b };

  super::validate_query_consistency(&joined);
  assert_eq!(
    joined.access(&(1, 10)),
    Some(("a".to_string(), "x".to_string()))
  );
  assert_eq!(
    joined.access(&(1, 20)),
    Some(("a".to_string(), "y".to_string()))
  );
  assert_eq!(
    joined.access(&(2, 10)),
    Some(("b".to_string(), "x".to_string()))
  );
  assert_eq!(
    joined.access(&(2, 20)),
    Some(("b".to_string(), "y".to_string()))
  );
  assert_eq!(joined.access(&(3, 10)), None);
}

#[test]
fn test_cross_join_empty() {
  let a: FastHashMap<u32, String> = FastHashMap::default();
  let mut b = FastHashMap::default();
  b.insert(10u32, "x".to_string());

  let joined = CrossJoinQuery { a, b };

  super::validate_query_consistency(&joined);
  // cross join with empty = empty
}

#[test]
fn test_cross_join_single() {
  let mut a = FastHashMap::default();
  a.insert(1u32, "only".to_string());

  let mut b = FastHashMap::default();
  b.insert(42u32, 99);

  let joined = CrossJoinQuery { a, b };

  super::validate_query_consistency(&joined);
  assert_eq!(joined.access(&(1, 42)), Some(("only".to_string(), 99)));
}
