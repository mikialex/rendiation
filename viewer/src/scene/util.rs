use std::{collections::HashMap, marker::PhantomData};
use std::{collections::HashSet, hash::Hash};

use arena::{Arena, Handle};

pub trait ResourcePair {
  type Data;
  type Resource;
  fn data(&self) -> &Self::Data;
  fn resource(&self) -> &Self::Resource;
  fn data_mut(&mut self) -> &mut Self::Data;
  fn resource_mut(&mut self) -> &mut Self::Resource;
}

pub struct WatchedArena<T> {
  arena: Arena<T>,
  modified: HashSet<Handle<T>>,
}

#[derive(Debug)]
pub enum SceneError {
  HandleCorrupted,
}

impl<T: ResourcePair> WatchedArena<T> {
  pub fn new() -> Self {
    Self {
      arena: Arena::new(),
      modified: HashSet::new(),
    }
  }

  pub fn drain_modified(&mut self) -> impl Iterator<Item = (&mut T, Handle<T>)> {
    // safety: modified is a set
    self.modified.drain().map(|h| {
      (
        unsafe { std::mem::transmute(self.arena.get_mut(h).unwrap()) },
        h,
      )
    })
  }

  pub fn get_data(&self, h: Handle<T>) -> Result<&T::Data, SceneError> {
    self
      .arena
      .get(h)
      .map(|v| v.data())
      .ok_or(SceneError::HandleCorrupted)
  }

  pub fn get_data_mut(&mut self, h: Handle<T>) -> Result<&mut T::Data, SceneError> {
    self.modified.insert(h);
    self
      .arena
      .get_mut(h)
      .map(|v| v.data_mut())
      .ok_or(SceneError::HandleCorrupted)
  }

  pub fn get_resource(&self, h: Handle<T>) -> Result<&T::Resource, SceneError> {
    self
      .arena
      .get(h)
      .map(|v| v.resource())
      .ok_or(SceneError::HandleCorrupted)
  }

  pub fn get_resource_mut(&mut self, h: Handle<T>) -> Result<&mut T::Resource, SceneError> {
    self
      .arena
      .get_mut(h)
      .map(|v| v.resource_mut())
      .ok_or(SceneError::HandleCorrupted)
  }

  pub fn insert(&mut self, v: T) -> Handle<T> {
    let h = self.arena.insert(v);
    self.modified.insert(h);
    h
  }

  pub fn remove(&mut self, h: Handle<T>) {
    self.modified.remove(&h);
    self.arena.remove(h);
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

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
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
