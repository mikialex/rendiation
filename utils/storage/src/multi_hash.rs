use crate::*;

/// this impl is to replace the HashMap<K, MultiHashSet<V>>
pub struct MultiHash<K, V> {
  guid: u64,
  keys: FastHashMap<K, (u64, u32)>,
  secondary: FastHashMap<(u64, u32), V>,
}

impl<K, V> Default for MultiHash<K, V> {
  fn default() -> Self {
    Self {
      guid: Default::default(),
      keys: Default::default(),
      secondary: Default::default(),
    }
  }
}

impl<K: Eq + std::hash::Hash, V: Eq> MultiHash<K, V> {
  pub fn shrink_to_fit(&mut self) {
    self.keys.shrink_to_fit();
    self.secondary.shrink_to_fit();
  }

  pub fn insert(&mut self, k: K, v: V) {
    self.guid += 1;
    let keys = self.keys.entry(k).or_insert_with(|| (self.guid, 0));
    self.secondary.insert(*keys, v);
    keys.1 += 1;
  }

  pub fn remove(&mut self, k: &K, v: &V) {
    if let Some((id, count)) = self.keys.get_mut(k) {
      for i in 0..*count {
        if let Some(v_stored) = self.secondary.get(&(*id, i)) {
          if v_stored == v {
            self.secondary.remove(&(*id, i)).unwrap();
            *count -= 1;
            // move the tail to fill the hole.
            if i != *count {
              let tail = self.secondary.remove(&(*id, *count)).unwrap();
              self.secondary.insert((*id, i), tail);
            }
          }
        }
      }
      if *count == 0 {
        self.keys.remove(k);
      }
    }
  }

  pub fn visit_multi(&self, k: &K, mut visitor: impl FnMut(&V)) {
    if let Some((id, count)) = self.keys.get(k) {
      for i in 0..*count {
        visitor(self.secondary.get(&(*id, i)).unwrap())
      }
    }
  }

  pub fn value_count(&self, k: &K) -> usize {
    let mut i = 0;
    self.visit_multi(k, |_| i += 1);
    i
  }
}

#[test]
fn test() {
  let mut map = MultiHash::<char, usize>::default();

  map.insert('a', 1);
  assert_eq!(map.value_count(&'a'), 1);

  map.insert('a', 1);
  assert_eq!(map.value_count(&'a'), 2);

  map.remove(&'a', &1);
  assert_eq!(map.value_count(&'a'), 1);

  map.insert('a', 2);
  assert_eq!(map.value_count(&'a'), 2);
}
