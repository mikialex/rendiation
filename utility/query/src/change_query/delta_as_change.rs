use crate::*;

#[derive(Clone)]
pub struct DeltaQueryAsChange<T>(pub T);

pub trait IntoDeltaQueryAsChangeExt: Sized {
  fn into_change(self) -> DeltaQueryAsChange<Self> {
    DeltaQueryAsChange(self)
  }
}
impl<T: Query> IntoDeltaQueryAsChangeExt for T {}

impl<T: CValue, Q: Query<Value = ValueChange<T>>> DataChanges for DeltaQueryAsChange<Q> {
  type Key = Q::Key;
  type Value = T;

  fn has_change(&self) -> bool {
    // iter_key_value may have heap allocation, use this to do a pre check
    // todo, we should add this to Query trait to avoid box
    if !self.0.has_item_hint() {
      return false;
    }

    self.0.iter_key_value().next().is_some()
  }

  fn iter_removed(&self) -> impl Iterator<Item = Self::Key> + '_ {
    self
      .0
      .iter_key_value()
      .filter_map(|(k, v)| v.is_removed().then_some(k))
  }

  fn iter_update_or_insert(&self) -> impl Iterator<Item = (Self::Key, Self::Value)> + '_ {
    self
      .0
      .iter_key_value()
      .filter_map(|v| v.1.new_value().map(|x| (v.0, x.clone())))
  }
}

#[test]
fn test_delta_query_as_change_empty() {
  let empty: FastHashMap<u32, ValueChange<String>> = FastHashMap::default();
  let change = DeltaQueryAsChange(empty);

  assert!(!change.has_change());
  assert_eq!(change.iter_removed().count(), 0);
  assert_eq!(change.iter_update_or_insert().count(), 0);
}

#[test]
fn test_delta_query_as_change_insert() {
  let mut delta = FastHashMap::default();
  delta.insert(1u32, ValueChange::Delta("a".to_string(), None));
  delta.insert(2, ValueChange::Delta("b".to_string(), None));

  let change = DeltaQueryAsChange(delta);

  assert!(change.has_change());

  let inserts: FastHashMap<u32, String> = change.iter_update_or_insert().collect();
  assert_eq!(inserts.len(), 2);
  assert_eq!(inserts[&1], "a");
  assert_eq!(inserts[&2], "b");

  assert_eq!(change.iter_removed().count(), 0);
}

#[test]
fn test_delta_query_as_change_update() {
  let mut delta = FastHashMap::default();
  delta.insert(
    1u32,
    ValueChange::Delta("new".to_string(), Some("old".to_string())),
  );

  let change = DeltaQueryAsChange(delta);

  assert!(change.has_change());
  assert_eq!(change.iter_removed().count(), 0);

  let updates: Vec<_> = change.iter_update_or_insert().collect();
  assert_eq!(updates.len(), 1);
  assert_eq!(updates[0], (1, "new".to_string()));
}

#[test]
fn test_delta_query_as_change_remove() {
  let mut delta = FastHashMap::default();
  delta.insert(1u32, ValueChange::Remove("removed".to_string()));
  delta.insert(2, ValueChange::Remove("also_removed".to_string()));

  let change = DeltaQueryAsChange(delta);

  assert!(change.has_change());

  let removed: FastHashSet<_> = change.iter_removed().collect();
  assert_eq!(removed.len(), 2);
  assert!(removed.contains(&1));
  assert!(removed.contains(&2));

  assert_eq!(change.iter_update_or_insert().count(), 0);
}

#[test]
fn test_delta_query_as_change_mixed() {
  let mut delta = FastHashMap::default();
  delta.insert(1u32, ValueChange::Delta("new".to_string(), None));
  delta.insert(2, ValueChange::Remove("gone".to_string()));

  let change = DeltaQueryAsChange(delta);

  assert!(change.has_change());

  let removed: Vec<_> = change.iter_removed().collect();
  assert_eq!(removed, vec![2]);

  let inserts: Vec<_> = change.iter_update_or_insert().collect();
  assert_eq!(inserts.len(), 1);
  assert_eq!(inserts[0], (1, "new".to_string()));
}
