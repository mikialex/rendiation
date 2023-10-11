use std::{
  any::{Any, TypeId},
  sync::{Arc, RwLock, RwLockReadGuard, Weak},
};

use fast_hash_collection::FastHashMap;
use storage::*;

use crate::*;

#[derive(Default)]
pub struct StorageGroup {
  pub storages: FastHashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

static GLOBAL_STORAGE_GROUPS: RwLock<Option<StorageGroup>> = RwLock::new(None);
pub fn setup_global_storage_group(sg: StorageGroup) -> Option<StorageGroup> {
  GLOBAL_STORAGE_GROUPS.write().unwrap().replace(sg)
}

struct SignalItem<T> {
  data: T,
  sub_event_handle: Option<ListHandle>,
  ref_count: u32,
  guid: u64, // weak semantics is impl by the guid compare in data access
}

pub struct IncrementalSignalGroupImpl<T: IncrementalBase> {
  data: RwLock<IndexReusedVec<SignalItem<T>>>,
  sub_watcher: RwLock<LinkListPool<EventListener<T::Delta>>>,
}

impl<T: IncrementalBase> Default for IncrementalSignalGroupImpl<T> {
  fn default() -> Self {
    Self {
      data: Default::default(),
      sub_watcher: Default::default(),
    }
  }
}

/// data storage point
#[derive(Clone)]
pub struct IncrementalSignalStorage<T: IncrementalBase> {
  inner: Arc<IncrementalSignalGroupImpl<T>>,
}

impl<T: IncrementalBase> IncrementalSignalStorage<T> {
  pub fn alloc(&self, data: T) -> IncrementalSignalPtr<T> {
    let mut storage = self.inner.data.write().unwrap();
    let guid = alloc_global_res_id();
    let data = SignalItem {
      data,
      sub_event_handle: None,
      ref_count: 1,
      guid,
    };
    let index = storage.insert(data);
    IncrementalSignalPtr {
      inner: Arc::downgrade(&self.inner),
      index,
      guid,
    }
  }
}

impl<T: IncrementalBase> Default for IncrementalSignalStorage<T> {
  fn default() -> Self {
    Self {
      inner: Default::default(),
    }
  }
}

/// data access point
pub struct IncrementalSignalPtr<T: IncrementalBase> {
  inner: Weak<IncrementalSignalGroupImpl<T>>,
  index: u32,
  guid: u64,
}

impl<T: IncrementalBase> GlobalIdentified for IncrementalSignalPtr<T> {
  fn guid(&self) -> u64 {
    self.guid
  }
}
impl<T: IncrementalBase> AsRef<dyn GlobalIdentified> for IncrementalSignalPtr<T> {
  fn as_ref(&self) -> &(dyn GlobalIdentified + 'static) {
    self
  }
}
impl<T: IncrementalBase> AsMut<dyn GlobalIdentified> for IncrementalSignalPtr<T> {
  fn as_mut(&mut self) -> &mut (dyn GlobalIdentified + 'static) {
    self
  }
}

impl<T: IncrementalBase> IncrementalSignalPtr<T> {
  pub fn new(data: T) -> Self {
    let id = data.type_id();

    let storages = GLOBAL_STORAGE_GROUPS.read().unwrap();
    let storages = storages
      .as_ref()
      .expect("global storage group not specified");
    if let Some(storage) = storages.storages.get(&id) {
      let storage = storage
        .downcast_ref::<IncrementalSignalStorage<T>>()
        .unwrap();
      storage.alloc(data)
    } else {
      let mut storages = GLOBAL_STORAGE_GROUPS.write().unwrap();
      let storages = storages
        .as_mut()
        .expect("global storage group not specified");
      let storage = storages
        .storages
        .entry(id)
        .or_insert_with(|| Box::<IncrementalSignalStorage<T>>::default());
      let storage = storage
        .downcast_ref::<IncrementalSignalStorage<T>>()
        .unwrap();
      storage.alloc(data)
    }
  }

  fn mutate_inner<R>(
    &self,
    f: impl FnOnce(&mut SignalItem<T>, &IncrementalSignalGroupImpl<T>) -> R,
  ) -> Option<R> {
    if let Some(inner) = self.inner.upgrade() {
      let mut storage = inner.data.write().unwrap();
      let data = storage.get_mut(self.index);
      if data.guid == self.guid {
        return Some(f(data, &inner));
      }
    }
    None
  }

  pub fn read(&self) -> Option<SignalPtrGuard<T>> {
    if let Some(inner) = self.inner.upgrade() {
      let storage = inner.data.read().unwrap();
      let data = storage.get(self.index);
      if data.guid == self.guid {
        // Safety, this ref to the self holder
        let storage = unsafe { std::mem::transmute(storage) };
        return Some(SignalPtrGuard {
          _holder: inner,
          inner: storage,
          index: self.index,
        });
      }
    }
    None
  }

  /// return should be removed from source after emitted
  fn on(&self, f: impl FnMut(&T::Delta) -> bool + Send + Sync + 'static) -> Option<u32> {
    self.mutate_inner(|data, inner| {
      let mut sub_watcher = inner.sub_watcher.write().unwrap();
      let watcher_handle = data
        .sub_event_handle
        .get_or_insert_with(|| sub_watcher.make_list());
      sub_watcher.insert(watcher_handle, Box::new(f))
    })
  }

  fn off(&self, token: u32) {
    self.mutate_inner(|data, inner| {
      let mut sub_watcher = inner.sub_watcher.write().unwrap();
      sub_watcher.remove(data.sub_event_handle.as_mut().unwrap(), token);
    });
  }

  pub fn mutate<R>(&mut self, mutator: impl FnOnce(Mutating<T>) -> R) -> Option<R> {
    self.mutate_with(mutator, |_| {})
  }

  pub fn mutate_with<R>(
    &mut self,
    mutator: impl FnOnce(Mutating<T>) -> R,
    mut extra_collector: impl FnMut(T::Delta),
  ) -> Option<R> {
    self.mutate_inner(|data, inner| {
      let mut sub_watcher = inner.sub_watcher.write().unwrap();

      mutator(Mutating {
        inner: &mut data.data,
        collector: &mut |delta| {
          // emit sub child
          sub_watcher.visit_and_remove(data.sub_event_handle.as_mut().unwrap(), |f, _| {
            (f(delta), true)
          });
          extra_collector(delta.clone())
        },
      })
    })
  }

  pub fn listen_by<N, C, U>(
    &self,
    mut mapper: impl FnMut(MaybeDeltaRef<T>, &dyn Fn(U)) + Send + Sync + 'static,
    channel_builder: &C,
  ) -> Option<impl Stream<Item = N>>
  where
    U: Send + Sync + 'static,
    C: ChannelLike<U, Message = N>,
  {
    let (sender, receiver) = channel_builder.build();

    let data = self.read()?;
    mapper(MaybeDeltaRef::All(&data), &|mapped| {
      C::send(&sender, mapped);
    });

    let remove_token = self.on(move |v| {
      mapper(MaybeDeltaRef::Delta(v), &|mapped| {
        C::send(&sender, mapped);
      });
      C::is_closed(&sender)
    })?;

    let dropper = IncrementalSignalStorageEventDropper {
      remove_token,
      weak: self.downgrade(),
    };
    Some(DropperAttachedStream::new(dropper, receiver))
  }
}

pub struct IncrementalSignalStorageEventDropper<T: IncrementalBase> {
  remove_token: u32,
  weak: IncrementalSignalWeakPtr<T>,
}

impl<T: IncrementalBase> Drop for IncrementalSignalStorageEventDropper<T> {
  fn drop(&mut self) {
    if let Some(source) = self.weak.upgrade() {
      source.off(self.remove_token);
    }
  }
}

impl<T: IncrementalBase> Clone for IncrementalSignalPtr<T> {
  fn clone(&self) -> Self {
    if let Some(inner) = self.inner.upgrade() {
      let mut storage = inner.data.write().unwrap();
      let data = storage.get_mut(self.index);
      if data.guid == self.guid {
        data.ref_count += 1;
      }
    }
    Self {
      inner: self.inner.clone(),
      index: self.index,
      guid: self.guid,
    }
  }
}

impl<T: IncrementalBase> Drop for IncrementalSignalPtr<T> {
  fn drop(&mut self) {
    if let Some(inner) = self.inner.upgrade() {
      let mut storage = inner.data.write().unwrap();
      let data = storage.get_mut(self.index);
      if data.guid == self.guid {
        data.ref_count -= 1;
        if data.ref_count == 0 {
          let removed = storage.remove(self.index);
          if let Some(list) = removed.sub_event_handle {
            inner.sub_watcher.write().unwrap().drop_list(list);
          }
        }
      }
    }
  }
}

impl<T: IncrementalBase> std::hash::Hash for IncrementalSignalPtr<T> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.guid.hash(state);
  }
}

impl<T: IncrementalBase> PartialEq for IncrementalSignalPtr<T> {
  fn eq(&self, other: &Self) -> bool {
    self.guid == other.guid
  }
}
impl<T: IncrementalBase> Eq for IncrementalSignalPtr<T> {}

impl<T: IncrementalBase + Default> Default for IncrementalSignalPtr<T> {
  fn default() -> Self {
    Self::new(T::default())
  }
}

impl<T: IncrementalBase> From<T> for IncrementalSignalPtr<T> {
  fn from(inner: T) -> Self {
    Self::new(inner)
  }
}

impl<T: IncrementalBase + Send + Sync> IncrementalBase for IncrementalSignalPtr<T> {
  type Delta = Self;

  fn expand(&self, mut cb: impl FnMut(Self::Delta)) {
    cb(self.clone())
  }
}

impl<T: ApplicableIncremental + Send + Sync> ApplicableIncremental for IncrementalSignalPtr<T> {
  type Error = T::Error;

  fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error> {
    *self = delta;
    Ok(())
  }
}

impl<T: IncrementalBase> IncrementalSignalPtr<T> {
  pub fn downgrade(&self) -> IncrementalSignalWeakPtr<T> {
    IncrementalSignalWeakPtr {
      inner: self.inner.clone(),
      index: self.index,
      guid: self.guid,
    }
  }
}

/// data access point
#[derive(Clone)]
pub struct IncrementalSignalWeakPtr<T: IncrementalBase> {
  inner: Weak<IncrementalSignalGroupImpl<T>>,
  index: u32,
  guid: u64,
}

impl<T: IncrementalBase> IncrementalSignalWeakPtr<T> {
  pub fn upgrade(&self) -> Option<IncrementalSignalPtr<T>> {
    if let Some(inner) = self.inner.upgrade() {
      let mut storage = inner.data.write().unwrap();
      let data = storage.get_mut(self.index);
      if data.guid == self.guid {
        data.ref_count += 1;
        return Some(IncrementalSignalPtr {
          inner: self.inner.clone(),
          index: self.index,
          guid: self.guid,
        });
      }
    }

    None
  }
}

pub struct SignalPtrGuard<'a, T: IncrementalBase> {
  _holder: Arc<IncrementalSignalGroupImpl<T>>,
  inner: RwLockReadGuard<'a, IndexReusedVec<SignalItem<T>>>,
  index: u32,
}

impl<'a, T: IncrementalBase> Deref for SignalPtrGuard<'a, T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    let inner = self.inner.deref();
    &inner.get(self.index).data
  }
}
