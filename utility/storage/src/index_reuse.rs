#[derive(Clone)]
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
  pub fn shrink_to_fit(&mut self) {
    let tail_size = self
      .storage
      .iter()
      .rev()
      .take_while(|v| v.is_none())
      .count();
    self.storage.truncate(self.storage.len() - tail_size);
    self.storage.shrink_to_fit();

    let new_len = self.storage.len();

    let new_empty = self
      .empty_list
      .iter()
      .cloned()
      .filter(|v| *v < new_len as u32)
      .collect();
    self.empty_list = new_empty;
    self.empty_list.shrink_to_fit();
  }

  pub fn insert(&mut self, data: T) -> u32 {
    if let Some(empty) = self.empty_list.pop() {
      self.storage[empty as usize] = data.into();
      empty
    } else {
      self.storage.push(data.into());
      self.storage.len() as u32 - 1
    }
  }

  pub fn iter(&self) -> impl Iterator<Item = (u32, &T)> {
    self
      .storage
      .iter()
      .enumerate()
      .filter_map(|(index, v)| Some((index as u32, v.as_ref()?)))
  }

  pub fn remove(&mut self, idx: u32) -> T {
    self.empty_list.push(idx);
    let idx = idx as usize;
    assert!(self.storage[idx].is_some());
    self.storage[idx].take().unwrap()
  }

  pub fn try_get_mut(&mut self, idx: u32) -> Option<&mut T> {
    self.storage.get_mut(idx as usize).and_then(|v| v.as_mut())
  }

  pub fn try_get(&self, idx: u32) -> Option<&T> {
    self.storage.get(idx as usize).and_then(|v| v.as_ref())
  }

  pub fn get_mut(&mut self, idx: u32) -> &mut T {
    self.try_get_mut(idx).expect("bad index")
  }

  pub fn get(&self, idx: u32) -> &T {
    self.try_get(idx).expect("bad index")
  }
}
