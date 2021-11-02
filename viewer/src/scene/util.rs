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

/// T should not contain interior mutability, it's logic error
impl<T> WatchedArena<T> {
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

  pub fn get(&self, h: Handle<T>) -> Result<&T, SceneError> {
    self.arena.get(h).ok_or(SceneError::HandleCorrupted)
  }

  pub fn get_mut(&mut self, h: Handle<T>) -> Result<&mut T, SceneError> {
    self.modified.insert(h);
    self.arena.get_mut(h).ok_or(SceneError::HandleCorrupted)
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

impl<T> Default for WatchedArena<T> {
  fn default() -> Self {
    Self::new()
  }
}

impl<T: ResourcePair> WatchedArena<T> {
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

  pub fn get_data(&self, h: Handle<T>) -> Result<&T::Data, SceneError> {
    self.get(h).map(|v| v.data())
  }

  pub fn get_data_mut(&mut self, h: Handle<T>) -> Result<&mut T::Data, SceneError> {
    self.get_mut(h).map(|v| v.data_mut())
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

#[derive(Debug, Copy, Hash, PartialEq, Eq)]
pub struct ValueID<T> {
  value: usize,
  ty: PhantomData<T>,
}

impl<T> Clone for ValueID<T> {
  fn clone(&self) -> Self {
    Self {
      value: self.value,
      ty: self.ty,
    }
  }
}

impl<T> Copy for ValueID<T> {}

impl<T> ValueIDGenerator<T>
where
  T: Eq + Hash + Clone,
{
  pub fn get_uuid(&mut self, v: &T) -> ValueID<T> {
    let count = self.inner.len();
    let id = self
      .inner
      .raw_entry_mut()
      .from_key(v)
      .or_insert_with(|| (v.clone(), count));
    ValueID {
      value: *id.1,
      ty: PhantomData,
    }
  }
}
