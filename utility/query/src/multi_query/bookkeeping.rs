use crate::*;

pub fn bookkeeping_hash_relation<K: CKey, V: CKey>(
  mapping: &mut FastHashMap<V, FastHashSet<K>>,
  changes: impl Query<Key = K, Value = ValueChange<V>>,
) {
  for (many, change) in changes.iter_key_value() {
    let new_one = change.new_value();

    let old_refed_one = change.old_value();
    // remove possible old relations
    if let Some(old_refed_one) = old_refed_one {
      let previous_one_refed_many = mapping.get_mut(old_refed_one).unwrap();
      previous_one_refed_many.remove(&many);

      if previous_one_refed_many.capacity() > previous_one_refed_many.len() * 2 {
        previous_one_refed_many.shrink_to_fit();
      }

      if previous_one_refed_many.is_empty() {
        mapping.remove(old_refed_one);
      }
    }

    // setup new relations
    if let Some(new_one) = new_one {
      let new_one_refed_many = mapping.entry(new_one.clone()).or_default();
      new_one_refed_many.insert(many.clone());
    }
  }

  if mapping.capacity() > mapping.len() * 2 {
    mapping.shrink_to_fit();
  }
}

#[derive(Clone)]
pub struct DenseIndexMapping<K, V> {
  mapping_buffer: LinkListPool<V>,
  // todo, this data representation is not optimal
  mapping: Vec<MappingEntry<K>>,
  large_mapping_fallback: FastHashMap<K, FastHashSet<V>>,
}

impl<K, V> DenseIndexMapping<K, V> {
  pub fn reserve(&mut self, additional_multi: usize, additional_one: usize) {
    self.mapping_buffer.reserve(additional_multi);
    self.mapping.reserve(additional_one);
  }

  pub fn memory_usage_no_indirect_in_bytes(&self) -> usize {
    self.mapping_buffer.memory_usage_no_indirect_in_bytes()
      + self.mapping.capacity() * std::mem::size_of::<MappingEntry<K>>()
      + self.large_mapping_fallback.capacity() * std::mem::size_of::<(K, FastHashSet<V>)>()
    // todo set it self is not computed
  }
}

impl<K, V> Default for DenseIndexMapping<K, V> {
  fn default() -> Self {
    Self {
      mapping_buffer: LinkListPool::default(),
      mapping: vec![],
      large_mapping_fallback: FastHashMap::default(),
    }
  }
}

#[derive(Default, Clone)]
enum MappingEntry<K> {
  List(ListHandle, K, u32),
  Fallback(K),
  #[default]
  Empty,
}

impl<K: CKey + LinearIdentified, V: CValue> MultiQuery for DenseIndexMapping<K, V> {
  type Key = K;
  type Value = V;
  fn iter_keys(&self) -> impl Iterator<Item = K> + '_ {
    let dense = self.mapping.iter().filter_map(|entry| match entry {
      MappingEntry::List(_, i, _) => Some(i.clone()),
      _ => None,
    });

    let fallback = self.large_mapping_fallback.keys().cloned();

    dense.chain(fallback)
  }

  fn access_multi(&self, o: &K) -> Option<impl Iterator<Item = V> + '_> {
    if let Some(entry) = self.mapping.get(o.alloc_index() as usize) {
      match entry {
        MappingEntry::List(list_handle, _, _) => EtherIter::B(
          self
            .mapping_buffer
            .iter_list(list_handle) // todo inject length info, impl size_hint
            .map(|(v, _)| v.clone()),
        )
        .into(),
        MappingEntry::Fallback(_) => {
          EtherIter::A(self.large_mapping_fallback.get(o).unwrap().iter().cloned()).into()
        }
        MappingEntry::Empty => None,
      }
    } else {
      None
    }
  }
}

pub fn bookkeeping_dense_index_relation<K: CKey + LinearIdentified, V: CKey + LinearIdentified>(
  mapping: &mut DenseIndexMapping<V, K>,
  changes: impl Query<Key = K, Value = ValueChange<V>>,
) {
  let changes_iter = changes.iter_key_value();
  // this change count contains remove, so the reserve may be too conservative
  let once_change_count = changes_iter.size_hint().0;
  // assume one change count equals multi change count
  // todo, try count exact change
  mapping.reserve(once_change_count, once_change_count);

  for (many, change) in changes_iter {
    let new_one = change.new_value();

    let old_refed_one = change.old_value();
    // remove possible old relations
    if let Some(old_refed_one) = old_refed_one {
      let entry = mapping
        .mapping
        .get_mut(old_refed_one.alloc_index() as usize)
        .unwrap();
      let mut should_remove = false;
      match entry {
        MappingEntry::List(previous_one_refed_many, _, len) => {
          //  this is O(n)
          mapping
            .mapping_buffer
            .visit_and_remove(previous_one_refed_many, |value, _| {
              let should_remove = *value == many;
              if should_remove {
                *len -= 1;
              }
              (should_remove, !should_remove)
            });

          if previous_one_refed_many.is_empty() {
            should_remove = true;
          }
        }
        MappingEntry::Fallback(previous_one_refed_many) => {
          let set = mapping
            .large_mapping_fallback
            .get_mut(previous_one_refed_many)
            .unwrap();
          set.remove(&many);
          if set.is_empty() {
            mapping
              .large_mapping_fallback
              .remove(previous_one_refed_many);
            should_remove = true;
          }
        }
        MappingEntry::Empty => unreachable!(),
      }
      if should_remove {
        *entry = MappingEntry::Empty;
      }
    }

    // setup new relations
    if let Some(new_one) = new_one {
      let alloc_index = new_one.alloc_index() as usize;
      if alloc_index >= mapping.mapping.len() {
        mapping.mapping.resize(alloc_index + 1, MappingEntry::Empty);
      }

      let m = &mut mapping.mapping[alloc_index];

      match m {
        MappingEntry::List(list_handle, _, len) => {
          mapping.mapping_buffer.insert(list_handle, many);
          *len += 1;

          if *len > 128 {
            let mut set =
              FastHashSet::with_capacity_and_hasher((*len as usize) * 2, Default::default());
            mapping
              .mapping_buffer
              .visit_and_remove(list_handle, |v, _| {
                set.insert(v.clone());
                (true, true)
              });
            mapping.large_mapping_fallback.insert(new_one.clone(), set);
            *m = MappingEntry::Fallback(new_one.clone());
          }
        }
        MappingEntry::Fallback(_) => {
          mapping
            .large_mapping_fallback
            .get_mut(new_one)
            .unwrap()
            .insert(many);
        }
        MappingEntry::Empty => {
          let mut handle = ListHandle::default();
          mapping.mapping_buffer.insert(&mut handle, many);
          *m = MappingEntry::List(handle, new_one.clone(), 1);
        }
      }
    }
  }
}

pub(super) enum EtherIter<A, B> {
  A(A),
  B(B),
}

impl<V, A: Iterator<Item = V>, B: Iterator<Item = V>> Iterator for EtherIter<A, B> {
  type Item = V;

  fn next(&mut self) -> Option<Self::Item> {
    match self {
      EtherIter::A(a) => a.next(),
      EtherIter::B(b) => b.next(),
    }
  }

  fn size_hint(&self) -> (usize, Option<usize>) {
    match self {
      EtherIter::A(a) => a.size_hint(),
      EtherIter::B(b) => b.size_hint(),
    }
  }
}

#[test]
fn test_bookkeeping_hash_relation_insert() {
  let mut mapping: FastHashMap<u32, FastHashSet<u32>> = FastHashMap::default();

  // insert: many 1->one 100, many 2->one 100
  let changes: FastHashMap<u32, ValueChange<u32>> = FastHashMap::from_iter([
    (1u32, ValueChange::Delta(100u32, None)),
    (2u32, ValueChange::Delta(100u32, None)),
  ]);

  bookkeeping_hash_relation(&mut mapping, &changes);

  assert_eq!(mapping.len(), 1);
  assert_eq!(mapping[&100].len(), 2);
  assert!(mapping[&100].contains(&1));
  assert!(mapping[&100].contains(&2));
}

#[test]
fn test_bookkeeping_hash_relation_update() {
  let mut mapping: FastHashMap<u32, FastHashSet<u32>> = FastHashMap::default();

  // setup: 1->100, 2->100
  let changes: FastHashMap<u32, ValueChange<u32>> = FastHashMap::from_iter([
    (1u32, ValueChange::Delta(100u32, None)),
    (2u32, ValueChange::Delta(100u32, None)),
  ]);
  bookkeeping_hash_relation(&mut mapping, &changes);

  // update: change 1->100 to 1->200
  let changes2: FastHashMap<u32, ValueChange<u32>> = FastHashMap::from_iter([
    (1u32, ValueChange::Delta(200u32, Some(100u32))),
  ]);
  bookkeeping_hash_relation(&mut mapping, &changes2);

  assert_eq!(mapping[&100].len(), 1);
  assert!(mapping[&100].contains(&2));
  assert_eq!(mapping[&200].len(), 1);
  assert!(mapping[&200].contains(&1));
}

#[test]
fn test_bookkeeping_hash_relation_remove() {
  let mut mapping: FastHashMap<u32, FastHashSet<u32>> = FastHashMap::default();

  // setup: 1->100, 2->100
  let changes: FastHashMap<u32, ValueChange<u32>> = FastHashMap::from_iter([
    (1u32, ValueChange::Delta(100u32, None)),
    (2u32, ValueChange::Delta(100u32, None)),
  ]);
  bookkeeping_hash_relation(&mut mapping, &changes);

  // remove: 1->100
  let changes2: FastHashMap<u32, ValueChange<u32>> =
    FastHashMap::from_iter([(1u32, ValueChange::Remove(100u32))]);
  bookkeeping_hash_relation(&mut mapping, &changes2);

  assert_eq!(mapping[&100].len(), 1);
  assert!(mapping[&100].contains(&2));

  // remove: 2->100 (last one for key 100)
  let changes3: FastHashMap<u32, ValueChange<u32>> =
    FastHashMap::from_iter([(2u32, ValueChange::Remove(100u32))]);
  bookkeeping_hash_relation(&mut mapping, &changes3);

  assert!(!mapping.contains_key(&100));
}

#[test]
fn test_dense_index_mapping_basic() {
  let mut mapping = DenseIndexMapping::<u32, u32>::default();

  // insert many -> one relations: 0->10, 1->10, 2->20
  let changes: FastHashMap<u32, ValueChange<u32>> = FastHashMap::from_iter([
    (0u32, ValueChange::Delta(10u32, None)),
    (1u32, ValueChange::Delta(10u32, None)),
    (2u32, ValueChange::Delta(20u32, None)),
  ]);
  bookkeeping_dense_index_relation(&mut mapping, &changes);

  super::validate_multi_query_consistency(&mapping);

  let many_for_10: FastHashSet<_> = mapping.access_multi(&10).unwrap().collect();
  assert_eq!(many_for_10.len(), 2);
  assert!(many_for_10.contains(&0));
  assert!(many_for_10.contains(&1));

  let many_for_20: FastHashSet<_> = mapping.access_multi(&20).unwrap().collect();
  assert_eq!(many_for_20.len(), 1);
  assert!(many_for_20.contains(&2));

  assert!(mapping.access_multi(&30).is_none());
}

#[test]
fn test_dense_index_mapping_upgrade_to_fallback() {
  let mut mapping = DenseIndexMapping::<u32, u32>::default();

  // create >128 entries all pointing to the same "one" (key 0)
  let count = 150u32;
  let changes: FastHashMap<u32, ValueChange<u32>> =
    (0..count).map(|i| (i, ValueChange::Delta(0u32, None))).collect();
  bookkeeping_dense_index_relation(&mut mapping, &changes);

  super::validate_multi_query_consistency(&mapping);

  let many: Vec<_> = mapping.access_multi(&0).unwrap().collect();
  assert_eq!(many.len(), count as usize);
  for i in 0..count {
    assert!(many.contains(&i));
  }
}

#[test]
fn test_dense_index_mapping_remove() {
  let mut mapping = DenseIndexMapping::<u32, u32>::default();

  // insert: 0->10, 1->10
  let changes: FastHashMap<u32, ValueChange<u32>> = FastHashMap::from_iter([
    (0u32, ValueChange::Delta(10u32, None)),
    (1u32, ValueChange::Delta(10u32, None)),
  ]);
  bookkeeping_dense_index_relation(&mut mapping, &changes);

  // remove: 0->10
  let changes2: FastHashMap<u32, ValueChange<u32>> =
    FastHashMap::from_iter([(0u32, ValueChange::Remove(10u32))]);
  bookkeeping_dense_index_relation(&mut mapping, &changes2);

  super::validate_multi_query_consistency(&mapping);

  let many_for_10: Vec<_> = mapping.access_multi(&10).unwrap().collect();
  assert_eq!(many_for_10.len(), 1);
  assert!(many_for_10.contains(&1));
}
