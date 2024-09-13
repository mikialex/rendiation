pub struct TokenedItemPackage<T> {
  inner: Vec<(u32, T)>,
  next_token: u32,
}

impl<T> Default for TokenedItemPackage<T> {
  fn default() -> Self {
    Self {
      inner: Default::default(),
      next_token: 0,
    }
  }
}

impl<T> TokenedItemPackage<T> {
  pub fn insert(&mut self, item: T) -> u32 {
    self.next_token += 1;
    self.inner.push((self.next_token, item));
    self.next_token
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
