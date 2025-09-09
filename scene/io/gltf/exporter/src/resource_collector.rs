use crate::*;

pub struct Resource<K, T> {
  pub collected: Vec<T>,
  pub mapping: FastHashMap<K, gltf_json::Index<T>>,
}

impl<K, T> Default for Resource<K, T> {
  fn default() -> Self {
    Self {
      collected: Default::default(),
      mapping: Default::default(),
    }
  }
}

impl<K, T> Resource<K, T> {
  pub fn append_and_skip_mapping(&mut self, v: T) -> gltf_json::Index<T> {
    let idx = self.collected.len();
    self.collected.push(v);
    gltf_json::Index::new(idx as u32)
  }

  pub fn mutate(&mut self, idx: gltf_json::Index<T>, f: impl FnOnce(&mut T)) {
    let v = &mut self.collected[idx.value()];
    f(v);
  }

  pub fn try_get(&self, key: &K) -> Option<gltf_json::Index<T>>
  where
    K: std::hash::Hash + Eq,
  {
    self.mapping.get(key).copied()
  }

  pub fn get(&self, key: &K) -> gltf_json::Index<T>
  where
    K: std::hash::Hash + Eq,
  {
    self.try_get(key).unwrap()
  }

  pub fn append(&mut self, key: K, v: T) -> gltf_json::Index<T>
  where
    K: std::hash::Hash + Eq,
  {
    let v = self.append_and_skip_mapping(v);
    self.mapping.insert(key, v);
    v
  }

  pub fn get_or_insert_with(&mut self, key: K, create: impl FnOnce() -> T) -> gltf_json::Index<T>
  where
    K: std::hash::Hash + Eq,
  {
    if let Some(v) = self.mapping.get(&key) {
      *v
    } else {
      self.append(key, create())
    }
  }
}
