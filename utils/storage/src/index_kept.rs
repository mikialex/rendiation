pub struct IndexKeptVec<T> {
  storage: Vec<Option<T>>,
}

impl<T> Default for IndexKeptVec<T> {
  fn default() -> Self {
    Self {
      storage: Default::default(),
    }
  }
}

impl<T> IndexKeptVec<T> {
  pub fn insert(&mut self, data: T, index: u32) {
    self
      .storage
      .reserve((index as usize - self.storage.len() + 1).max(0));

    while self.storage.len() <= index as usize {
      self.storage.push(None)
    }
    self.storage[index as usize] = Some(data);
  }

  pub fn iter(&self) -> impl Iterator<Item = (u32, &T)> {
    self
      .storage
      .iter()
      .enumerate()
      .filter_map(|(index, v)| Some((index as u32, v.as_ref()?)))
  }

  pub fn remove(&mut self, idx: u32) -> T {
    let idx = idx as usize;
    assert!(self.storage[idx].is_some());
    self.storage[idx].take().unwrap()
  }

  pub fn try_get_mut(&mut self, idx: u32) -> Option<&mut T> {
    self.storage[idx as usize].as_mut()
  }

  pub fn try_get(&self, idx: u32) -> Option<&T> {
    self.storage[idx as usize].as_ref()
  }

  pub fn get_mut(&mut self, idx: u32) -> &mut T {
    self.try_get_mut(idx).expect("bad index")
  }

  pub fn get(&self, idx: u32) -> &T {
    self.try_get(idx).expect("bad index")
  }
}
