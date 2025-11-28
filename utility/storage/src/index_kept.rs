#[derive(Clone)]
pub struct IndexKeptVec<T> {
  storage: Vec<Option<T>>,
  len: usize,
}

impl<T> Default for IndexKeptVec<T> {
  fn default() -> Self {
    Self {
      storage: Default::default(),
      len: 0,
    }
  }
}

impl<T> IndexKeptVec<T> {
  pub fn shrink_to_fit(&mut self) {
    let tail_size = self
      .storage
      .iter()
      .rev()
      .take_while(|v| v.is_none())
      .count();
    self.storage.truncate(self.storage.len() - tail_size);
    self.storage.shrink_to_fit()
  }

  pub fn len(&self) -> usize {
    self.len
  }

  pub fn is_empty(&self) -> bool {
    self.len == 0
  }

  pub fn grow_to(&mut self, len: usize) {
    let new_len = len.max(self.storage.len());
    self.storage.resize_with(new_len, || None);
  }

  pub fn insert(&mut self, index: usize, data: T) {
    self.grow_to(index + 1);
    if self.storage[index].is_none() {
      self.len += 1;
    }
    self.storage[index] = Some(data);
  }

  pub fn iter(&self) -> impl Iterator<Item = (usize, &T)> {
    self
      .storage
      .iter()
      .enumerate()
      .filter_map(|(index, v)| Some((index, v.as_ref()?)))
  }

  pub fn remove(&mut self, idx: usize) -> Option<T> {
    let r = self.storage[idx].take();

    if r.is_some() {
      self.len -= 1;
    }

    r
  }

  pub fn get_insert_with(&mut self, idx: usize, f: impl FnOnce() -> T) -> &mut T {
    self.grow_to(idx + 1);
    let store = unsafe { self.storage.get_unchecked_mut(idx) };
    store.get_or_insert_with(f)
  }

  pub fn try_get_mut_ref(&mut self, idx: usize) -> Option<&mut Option<T>> {
    self.storage.get_mut(idx)
  }

  pub fn try_get_mut(&mut self, idx: usize) -> Option<&mut T> {
    self.storage.get_mut(idx).and_then(|v| v.as_mut())
  }

  pub fn try_get(&self, idx: usize) -> Option<&T> {
    self.storage.get(idx).and_then(|v| v.as_ref())
  }

  pub fn get_mut(&mut self, idx: usize) -> &mut T {
    self.try_get_mut(idx).expect("bad index")
  }

  pub fn get(&self, idx: usize) -> &T {
    self.try_get(idx).expect("bad index")
  }
}
