use std::hash::Hash;

use crate::*;

/// a hashmap supports fast iteration at cost of access performance and key duplication
///
/// rust's std hashmap or hashbrown is slow to iter compare to simple vec
#[derive(Clone)]
pub struct FastIterMap<K, V> {
  data: Vec<(K, V)>,
  mapping: FastHashMap<K, usize>,
}

impl<K: Eq + Hash, V> FastIterMap<K, V> {
  #[inline(always)]
  pub fn get(&self, k: &K) -> Option<&V> {
    let index = self.mapping.get(k)?;
    let v = unsafe { self.data.get_unchecked(*index) };
    Some(&v.1)
  }

  pub fn iter(&self) -> impl Iterator<Item = &(K, V)> {
    self.data.iter()
  }
}

impl<K: Eq + Hash + Clone, V> FromIterator<(K, V)> for FastIterMap<K, V> {
  fn from_iter<T: IntoIterator<Item = (K, V)>>(iter: T) -> Self {
    let iter = iter.into_iter();
    let size_hint = iter.size_hint();
    let capacity = size_hint.1.unwrap_or(size_hint.0);
    let mut mapping = FastHashMap::with_capacity_and_hasher(capacity, FastHasherBuilder::default());
    let mut data = Vec::with_capacity(capacity);
    for (k, v) in iter {
      if mapping.insert(k.clone(), data.len()).is_none() {
        data.push((k, v));
      }
    }
    data.shrink_to_fit();
    mapping.shrink_to_fit();
    Self { data, mapping }
  }
}
