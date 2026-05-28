use crate::*;

#[derive(Clone)]
pub struct ChainQuery<R, U> {
  pub first: R,
  pub next: U,
}

impl<U, R> Query for ChainQuery<R, U>
where
  U: Query,
  R: Query<Value = U::Key>,
{
  type Key = R::Key;
  type Value = U::Value;
  fn iter_key_value(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    self
      .first
      .iter_key_value()
      .filter_map(|(k, v)| self.next.access(&v).map(|v2| (k, v2)))
  }

  fn access(&self, key: &Self::Key) -> Option<Self::Value> {
    let o = self.first.access(key)?;
    self.next.access(&o)
  }

  fn has_item_hint(&self) -> bool {
    self.first.has_item_hint() || self.next.has_item_hint()
  }
}

#[test]
fn test_chain_query() {
  // first: item_id -> category_id
  let mut items = FastHashMap::default();
  items.insert(1u32, 100u32);
  items.insert(2, 200);
  items.insert(3, 100);

  // next: category_id -> category_name
  let mut categories = FastHashMap::default();
  categories.insert(100u32, "cat_a".to_string());
  categories.insert(200, "cat_b".to_string());

  let chained = ChainQuery {
    first: items,
    next: categories,
  };

  super::validate_query_consistency(&chained);
  assert_eq!(chained.access(&1), Some("cat_a".to_string()));
  assert_eq!(chained.access(&2), Some("cat_b".to_string()));
  assert_eq!(chained.access(&3), Some("cat_a".to_string()));
  assert_eq!(chained.access(&4), None);
}

#[test]
fn test_chain_query_broken_link() {
  // first maps to keys that don't exist in next
  let mut items = FastHashMap::default();
  items.insert(1u32, 999u32); // 999 doesn't exist in categories

  let categories: FastHashMap<u32, String> = FastHashMap::default();

  let chained = ChainQuery {
    first: items,
    next: categories,
  };

  super::validate_query_consistency(&chained);
  assert_eq!(chained.access(&1), None);
}

#[test]
fn test_chain_query_empty_first() {
  let items: FastHashMap<u32, u32> = FastHashMap::default();
  let mut categories = FastHashMap::default();
  categories.insert(100u32, "cat_a".to_string());

  let chained = ChainQuery {
    first: items,
    next: categories,
  };

  super::validate_query_consistency(&chained);
  assert_eq!(chained.access(&1), None);
}
