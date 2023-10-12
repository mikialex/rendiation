pub struct IndexReusedVec<T> {
  storage: Vec<Option<T>>,
  empty_list: Vec<u32>,
}

impl<T> Default for IndexReusedVec<T> {
  fn default() -> Self {
    Self {
      storage: Default::default(),
      empty_list: Default::default(),
    }
  }
}

impl<T> IndexReusedVec<T> {
  pub fn insert(&mut self, data: T) -> u32 {
    if let Some(empty) = self.empty_list.pop() {
      self.storage[empty as usize] = data.into();
      empty
    } else {
      self.storage.push(data.into());
      self.storage.len() as u32 - 1
    }
  }

  pub fn remove(&mut self, idx: u32) -> T {
    self.empty_list.push(idx);
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
