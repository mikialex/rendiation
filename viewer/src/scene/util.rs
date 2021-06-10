use std::{collections::HashMap, marker::PhantomData};
use std::{collections::HashSet, hash::Hash};

use arena::{Arena, Handle};

pub struct WatchedArena<T> {
  arena: Arena<T>,
  modified: HashSet<Handle<T>>,
}

#[derive(Debug)]
pub enum SceneError {
  HandleCorrupted,
}

impl<T> WatchedArena<T> {
  pub fn new() -> Self {
    Self {
      arena: Arena::new(),
      modified: HashSet::new(),
    }
  }

  pub fn get(&self, h: Handle<T>) -> Result<&T, SceneError> {
    self.arena.get(h).ok_or(SceneError::HandleCorrupted)
  }

  pub fn get_mut_not_record_change(&mut self, h: Handle<T>) -> Result<&mut T, SceneError> {
    self.arena.get_mut(h).ok_or(SceneError::HandleCorrupted)
  }

  pub fn mutate(&mut self, h: Handle<T>) -> Result<&mut T, SceneError> {
    self.modified.insert(h);
    self.arena.get_mut(h).ok_or(SceneError::HandleCorrupted)
  }

  pub fn insert(&mut self, v: T) -> Handle<T> {
    self.arena.insert(v)
  }

  pub fn remove(&mut self, handle: Handle<T>) {
    self.arena.remove(handle);
  }
}

pub struct ValueIDGenerator<T> {
  inner: HashMap<T, usize>,
}

impl<T> Default for ValueIDGenerator<T> {
  fn default() -> Self {
    Self {
      inner: HashMap::default(),
    }
  }
}

pub struct ValueID<T> {
  value: usize,
  ty: PhantomData<T>,
}

impl<T> ValueIDGenerator<T>
where
  T: Eq + Hash,
{
  pub fn get_uuid(&mut self, v: T) -> ValueID<T> {
    let count = self.inner.len();
    let id = self.inner.entry(v).or_insert(count);
    ValueID {
      value: *id,
      ty: PhantomData,
    }
  }
}
