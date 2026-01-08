use std::marker::PhantomData;

pub struct GenerationalArena<T> {
  storage: Vec<(Option<T>, u64)>,
  low_position_empties: Vec<usize>,
  high_position_empties: Vec<usize>,
  pub on_capacity_change: Option<Box<dyn FnMut(usize) + Send + Sync>>,
}

impl<T> Default for GenerationalArena<T> {
  fn default() -> Self {
    Self::with_capacity(32)
  }
}

impl<T> GenerationalArena<T> {
  pub fn with_capacity(capacity: usize) -> Self {
    let capacity = capacity.max(32);
    let half = capacity / 2;
    let low_position_empties = (0..half).collect();
    let high_position_empties = (half..capacity).collect();

    GenerationalArena {
      storage: (0..capacity).map(|_| (None, 0)).collect(),
      low_position_empties,
      high_position_empties,
      on_capacity_change: None,
    }
  }

  #[inline(always)]
  fn pop_empty(&mut self) -> Option<usize> {
    if let Some(idx) = self.low_position_empties.pop() {
      idx.into()
    } else if let Some(idx) = self.high_position_empties.pop() {
      idx.into()
    } else {
      None
    }
  }

  #[inline(never)]
  fn grow(&mut self) {
    let old_len = self.storage.len();
    let next_capacity = old_len * 2;

    self.storage.resize_with(next_capacity, || (None, 0));

    let new_half = next_capacity / 2;
    let additional = new_half - old_len;
    self.low_position_empties.reserve_exact(additional);
    self.high_position_empties.reserve_exact(additional);

    self.high_position_empties.retain(|v| {
      if v < &new_half {
        self.low_position_empties.push(*v);
        false
      } else {
        true
      }
    });

    for i in old_len..next_capacity {
      self.high_position_empties.push(i);
    }

    if let Some(f) = self.on_capacity_change.as_mut() {
      f(next_capacity)
    }
  }

  #[inline(never)]
  fn shrink(&mut self) {
    if self.storage.len() <= 32 {
      return;
    }

    let old_len = self.storage.len();
    let next_capacity = old_len / 2;

    self.storage.truncate(next_capacity);
    self.storage.shrink_to_fit();

    let new_half = next_capacity / 2;

    self.low_position_empties.retain(|v| {
      if v >= &new_half {
        self.high_position_empties.push(*v);
        false
      } else {
        true
      }
    });

    self.low_position_empties.shrink_to_fit();
    self.high_position_empties.shrink_to_fit();

    if let Some(f) = self.on_capacity_change.as_mut() {
      f(next_capacity)
    }

    if self.high_position_empties.len() == self.high_position_empties.capacity() {
      self.shrink();
    }
  }

  pub fn insert(&mut self, value: T) -> Handle<T> {
    let index = if let Some(idx) = self.pop_empty() {
      idx
    } else {
      self.grow();
      self.pop_empty().unwrap()
    };

    let item = unsafe { self.storage.get_unchecked_mut(index) };

    item.0 = Some(value);
    item.1 += 1;

    Handle {
      index,
      generation: item.1,
      phantom: PhantomData,
    }
  }

  pub fn remove(&mut self, handle: Handle<T>) -> Option<T> {
    let item = self.storage.get_mut(handle.index)?;
    if item.1 != handle.generation {
      return None;
    }

    let r = item.0.take()?;

    let index = handle.index;
    if index >= self.high_position_empties.capacity() {
      self.high_position_empties.push(index);
      if self.high_position_empties.len() == self.high_position_empties.capacity() {
        self.shrink();
      }
    } else {
      self.low_position_empties.push(index);
    }

    Some(r)
  }

  pub fn get(&self, handle: Handle<T>) -> Option<&T> {
    let r = self.storage.get(handle.index)?;
    if r.1 != handle.generation {
      return None;
    }
    r.0.as_ref()
  }

  pub fn get_mut(&mut self, handle: Handle<T>) -> Option<&mut T> {
    let r = self.storage.get_mut(handle.index)?;
    if r.1 != handle.generation {
      return None;
    }
    r.0.as_mut()
  }

  //   pub fn iter(&self) -> impl Iterator<Item = (&T, Handle<T>)> {}
}

pub struct Handle<T> {
  index: usize,
  generation: u64,
  phantom: PhantomData<T>,
}

#[test]
fn arena() {
  let mut a: GenerationalArena<usize> = Default::default();

  let h = a.insert(42);
  assert_eq!(a.remove(h), Some(42));

  let h = a.insert(42);
  assert_eq!(h.generation, 2);

  let mut handles = Vec::new();

  handles.push(h);

  for i in 0..100 {
    let h = a.insert(i);
    handles.push(h);
  }

  assert_eq!(a.storage.len(), 128);

  for h in handles {
    a.remove(h).unwrap();
  }

  assert_eq!(a.storage.len(), 32);
}
