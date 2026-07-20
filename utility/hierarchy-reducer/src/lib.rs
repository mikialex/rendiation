use std::hash::Hash;

use fast_hash_collection::FastHashMap;

#[cfg(test)]
mod test;

pub struct HierarchyMonoidReducer<K, T> {
  tree: Vec<Option<T>>,
  dirty_flags: Vec<bool>,
  dirty_indices: Vec<usize>,
  mapping: FastHashMap<K, usize>,
  /// reverse mapping, used in notify remove swap remove
  leaf_keys: Vec<Option<K>>,
  count: usize,
  pot: usize,
}

impl<K, T> Default for HierarchyMonoidReducer<K, T> {
  fn default() -> Self {
    Self {
      tree: vec![None, None],
      dirty_flags: vec![false, false],
      dirty_indices: Vec::new(),
      mapping: FastHashMap::default(),
      leaf_keys: vec![None],
      count: 0,
      pot: 1,
    }
  }
}

impl<K, T> HierarchyMonoidReducer<K, T>
where
  K: Hash + Eq + Clone,
  T: Clone,
{
  fn mark_dirty(&mut self, idx: usize) {
    if !self.dirty_flags[idx] {
      self.dirty_flags[idx] = true;
      self.dirty_indices.push(idx - self.pot);
    }
  }

  // remove none exist is allowed
  pub fn notify_remove(&mut self, key: &K) {
    let Some(offset) = self.mapping.remove(key) else {
      return;
    };
    let leaf_idx = self.pot + offset;
    let last = self.count - 1;

    if offset != last {
      let last_leaf = self.pot + last;
      self.tree[leaf_idx] = self.tree[last_leaf].take();
      self.tree[last_leaf] = None;
      let moved_key = self.leaf_keys[last].take().unwrap();
      self.leaf_keys[offset] = Some(moved_key.clone());
      self.mapping.insert(moved_key, offset);
      self.mark_dirty(leaf_idx);
      self.mark_dirty(last_leaf);
    } else {
      self.tree[leaf_idx] = None;
      self.leaf_keys[offset] = None;
      self.mark_dirty(leaf_idx);
    }

    self.count -= 1;
  }

  pub fn notify_insert_or_update(&mut self, key: K, value: T) {
    if let Some(&offset) = self.mapping.get(&key) {
      let leaf = self.pot + offset;
      self.tree[leaf] = Some(value);
      self.mark_dirty(leaf);
    } else {
      if self.count == self.pot {
        self.grow();
      }
      let leaf = self.pot + self.count;
      self.tree[leaf] = Some(value);
      self.leaf_keys[self.count] = Some(key.clone());
      self.mapping.insert(key, self.count);
      self.mark_dirty(leaf);
      self.count += 1;
    }
  }

  fn grow(&mut self) {
    let new_pot = self.pot * 2;
    let mut new_tree = vec![None; 2 * new_pot];
    let mut new_flags = vec![false; 2 * new_pot];

    // preserve old internal nodes: old i -> new 2*i (left subtree of new root)
    for i in 1..self.pot {
      new_tree[2 * i] = self.tree[i].take();
    }

    // copy old leaves
    for i in 0..self.pot {
      new_tree[new_pot + i] = self.tree[self.pot + i].take();
    }

    // mapping and dirty_indices store offsets, unchanged by pot growth
    for &offset in &self.dirty_indices {
      new_flags[new_pot + offset] = true;
    }

    // new root = old root (left child of new root), right subtree is all None
    new_tree[1] = new_tree[2].clone();

    self.leaf_keys.resize(new_pot, None);
    self.tree = new_tree;
    self.dirty_flags = new_flags;
    self.pot = new_pot;
  }

  // caller guarantees: dirty_indices is empty and tree is fully up-to-date
  fn shrink(&mut self) {
    let new_pot = (self.pot / 2).max(1);
    let mut new_tree = vec![None; 2 * new_pot];

    // copy leaves (contiguous range, first new_pot leaves)
    for i in 0..new_pot {
      new_tree[new_pot + i] = self.tree[self.pot + i].take();
    }

    // copy internals: new tree = old left subtree (root at old node 2)
    fn map_old_idx(i: usize) -> usize {
      if i == 1 {
        2
      } else if i % 2 == 0 {
        2 * map_old_idx(i / 2)
      } else {
        2 * map_old_idx(i / 2) + 1
      }
    }
    for i in 1..new_pot {
      new_tree[i] = self.tree[map_old_idx(i)].take();
    }

    self.leaf_keys.truncate(new_pot);
    self.leaf_keys.shrink_to_fit();
    self.tree = new_tree;
    self.dirty_flags = vec![false; 2 * new_pot];
    self.pot = new_pot;
  }

  fn combine(left: Option<T>, right: Option<T>, reducer: &impl Fn(T, T) -> T) -> Option<T> {
    match (left, right) {
      (None, None) => None,
      (Some(x), None) | (None, Some(x)) => Some(x),
      (Some(x), Some(y)) => Some(reducer(x, y)),
    }
  }

  // return the last reduced result without recomputing
  pub fn current_value(&self) -> Option<&T> {
    self.tree[1].as_ref()
  }

  pub fn update(&mut self, reducer: impl Fn(T, T) -> T) -> Option<T> {
    if self.count == 0 {
      return None;
    }

    let mut current = std::mem::take(&mut self.dirty_indices);
    // convert offsets to absolute leaf indices
    for offset in &mut current {
      *offset += self.pot;
    }

    // layer-by-layer propagation
    loop {
      let mut next = Vec::new();

      for &idx in &current {
        self.dirty_flags[idx] = false;

        if idx >= self.pot {
          // leaf
          if idx >= self.pot + self.count {
            self.tree[idx] = None;
          }
        } else {
          // internal
          let left = self.tree[2 * idx].clone();
          let right = self.tree[2 * idx + 1].clone();
          self.tree[idx] = Self::combine(left, right, &reducer);
        }

        if idx > 1 {
          let parent = idx / 2;
          if !self.dirty_flags[parent] {
            self.dirty_flags[parent] = true;
            next.push(parent);
          }
        }
      }

      if next.is_empty() {
        break;
      }
      current = next;
    }

    // shrink after update, tree is fully consistent and dirty_indices is empty
    if self.count < self.pot / 2 && self.pot > 1 {
      self.shrink();
    }

    self.tree[1].clone()
  }
}
