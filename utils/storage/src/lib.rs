use std::marker::PhantomData;

pub struct Storage<T, S: StorageBehavior<T>> {
  data: S::Container,
}
pub struct Handle<T, S: StorageBehavior<T>> {
  phantom: PhantomData<S>,
  phantom_t: PhantomData<T>,
  handle: S::Handle,
}

impl<T, S: StorageBehavior<T>> Clone for Handle<T, S> {
  fn clone(&self) -> Self {
    Self::new(self.handle)
  }
}

impl<T, S: StorageBehavior<T>> Handle<T, S> {
  pub fn new(handle: S::Handle) -> Self {
    Self {
      phantom: PhantomData,
      phantom_t: PhantomData,
      handle,
    }
  }
}

pub trait StorageBehavior<T>: Sized {
  type Container: Default;
  type Handle: Copy;

  fn insert(c: &mut Self::Container, v: T) -> Handle<T, Self>;
  fn get(c: &Self::Container, handle: Self::Handle) -> Option<&T>;
  fn get_mut(c: &mut Self::Container, handle: Self::Handle) -> Option<&mut T>;
  fn size(c: &Self::Container) -> usize;
}

impl<T, S: StorageBehavior<T>> Storage<T, S> {
  pub fn new() -> Self {
    Self {
      data: S::Container::default(),
    }
  }

  pub fn insert(&mut self, v: T) -> Handle<T, S> {
    S::insert(&mut self.data, v)
  }

  pub fn get(&self, h: Handle<T, S>) -> Option<&T> {
    S::get(&self.data, h.handle)
  }

  pub fn get_mut(&mut self, h: Handle<T, S>) -> Option<&mut T> {
    S::get_mut(&mut self.data, h.handle)
  }

  pub fn contains(&self, h: Handle<T, S>) -> bool {
    S::get(&self.data, h.handle).is_some()
  }

  pub fn size(&self) -> usize {
    S::size(&self.data)
  }
}

pub struct VecStorage;

impl<T> StorageBehavior<T> for VecStorage {
  type Container = Vec<T>;
  type Handle = usize;

  fn insert(c: &mut Self::Container, v: T) -> Handle<T, Self> {
    c.push(v);
    Handle::new(c.len() - 1)
  }
  fn get(c: &Self::Container, handle: Self::Handle) -> Option<&T> {
    c.get(handle)
  }
  fn get_mut(c: &mut Self::Container, handle: Self::Handle) -> Option<&mut T> {
    c.get_mut(handle)
  }
  fn size(c: &Self::Container) -> usize {
    c.len()
  }
}

pub struct DeduplicateVecStorage;
impl<T: PartialEq + Copy> StorageBehavior<T> for DeduplicateVecStorage {
  type Container = Vec<T>;
  type Handle = usize;

  fn insert(c: &mut Self::Container, v: T) -> Handle<T, Self> {
    c.push(v);
    let index = c.iter().position(|&cv| cv == v).unwrap_or_else(|| {
      c.push(v);
      c.len() - 1
    });
    Handle::new(index)
  }

  fn get(c: &Self::Container, handle: Self::Handle) -> Option<&T> {
    c.get(handle)
  }
  fn get_mut(c: &mut Self::Container, handle: Self::Handle) -> Option<&mut T> {
    c.get_mut(handle)
  }
  fn size(c: &Self::Container) -> usize {
    c.len()
  }
}

pub struct EpochVecStorage;
pub struct EpochItem<T> {
  epoch: u64,
  item: Option<T>,
}

pub struct EpochVecStorageImpl<T> {
  inner: Vec<EpochItem<T>>,
  free_list: Vec<usize>,
}
impl<T> Default for EpochVecStorageImpl<T> {
  fn default() -> Self {
    Self {
      inner: Vec::new(),
      free_list: Vec::new(),
    }
  }
}

#[derive(Clone, Copy)]
pub struct EpochHandle {
  handle: usize,
  epoch: u64,
}
impl<T> StorageBehavior<T> for EpochVecStorage {
  type Container = EpochVecStorageImpl<T>;
  type Handle = EpochHandle;

  fn insert(c: &mut Self::Container, v: T) -> Handle<T, Self> {
    if let Some(position) = c.free_list.pop() {
      let mut old = &mut c.inner[position];
      old.item = v.into();
      Handle::new(EpochHandle {
        handle: position,
        epoch: old.epoch + 1,
      })
    } else {
      let handle = c.inner.len();
      c.inner.push(EpochItem {
        epoch: 0,
        item: v.into(),
      });
      Handle::new(EpochHandle { handle, epoch: 0 })
    }
  }

  fn get(c: &Self::Container, handle: Self::Handle) -> Option<&T> {
    let store = c.inner.get(handle.handle)?;
    let value = store.item.as_ref()?;
    (store.epoch == handle.epoch).then(|| value)
  }

  fn get_mut(c: &mut Self::Container, handle: Self::Handle) -> Option<&mut T> {
    let store = c.inner.get_mut(handle.handle)?;
    let value = store.item.as_mut()?;
    (store.epoch == handle.epoch).then(|| value)
  }

  fn size(c: &Self::Container) -> usize {
    c.inner.len() - c.free_list.len()
  }
}
