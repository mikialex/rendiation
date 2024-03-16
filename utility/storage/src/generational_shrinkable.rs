pub struct GenerationalShrinkableVec<T> {
  inner: Vec<(u32, T)>,
  next_id: u32,
}

impl<T> Default for GenerationalShrinkableVec<T> {
  fn default() -> Self {
    Self {
      inner: Default::default(),
      next_id: 0,
    }
  }
}

impl<T> GenerationalShrinkableVec<T> {
  pub fn insert(&mut self, item: T) -> u32 {
    self.next_id += 1;
    self.inner.push((self.next_id, item));
    self.next_id
  }

  pub fn remove(&mut self, handle: u32) {
    let idx = self
      .inner
      .iter()
      .position(|v| v.0 == handle)
      .expect("event source remove failed");
    let _ = self.inner.swap_remove(idx);
  }

  pub fn iter_remove_if(&mut self, f: impl Fn(&mut T) -> bool) {
    let mut idx = 0;
    while idx < self.inner.len() {
      let item = &mut self.inner[idx].1;
      if f(item) {
        self.inner.swap_remove(idx);
      } else {
        idx += 1;
      }
    }
  }
}
