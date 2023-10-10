use std::sync::{Arc, RwLock, RwLockReadGuard, Weak};

use storage::*;

use crate::*;

struct SignalItem<T> {
  data: T,
  sub_event_handle: Option<ListHandle>, // todo, drop list
  ref_count: u32,
  guid: u64, // weak semantics is impl by the guid compare in data access
}

pub struct IncrementalSignalGroupImpl<T: IncrementalBase> {
  data: RwLock<IndexReusedVec<SignalItem<T>>>,
  group_watcher: EventSource<T::Delta>,
  sub_watcher: RwLock<LinkListPool<EventListener<T::Delta>>>,
}

impl<T: IncrementalBase> Default for IncrementalSignalGroupImpl<T> {
  fn default() -> Self {
    Self {
      data: Default::default(),
      group_watcher: Default::default(),
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

  /// return should be removed from source after emitted
  pub fn on_all_change(
    &self,
    f: impl FnMut(&T::Delta) -> bool + Send + Sync + 'static,
  ) -> RemoveToken<T::Delta> {
    self.inner.group_watcher.on(f)
  }

  pub fn off_all_change(&self, token: RemoveToken<T::Delta>) {
    self.inner.group_watcher.off(token)
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

impl<T: IncrementalBase> IncrementalSignalPtr<T> {
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

  fn emit(&self, delta: &T::Delta) {
    self.mutate_inner(|data, inner| {
      let mut sub_watcher = inner.sub_watcher.write().unwrap();
      sub_watcher.visit_and_remove(data.sub_event_handle.as_mut().unwrap(), |f| f(delta))
    });
  }

  // pub fn listen_by<N, C, U>(
  //   &self,
  //   mut mapper: impl FnMut(MaybeDeltaRef<T>, &dyn Fn(U)) + Send + Sync + 'static,
  //   channel_builder: &C,
  // ) -> Option<impl Stream<Item = N>>
  // where
  //   U: Send + Sync + 'static,
  //   C: ChannelLike<U, Message = N>,
  // {
  //   let (sender, receiver) = channel_builder.build();

  //   let data = self.read()?;
  //   mapper(MaybeDeltaRef::All(&data), &|mapped| {
  //     C::send(&sender, mapped);
  //   });

  //   let remove_token = self.on(move |v| {
  //     mapper(MaybeDeltaRef::Delta(v), &|mapped| {
  //       C::send(&sender, mapped);
  //     });
  //     C::is_closed(&sender)
  //   });

  //   let dropper = EventSourceDropper::new(remove_token, self.delta_source.make_weak());
  //   EventSourceStream::new(dropper, receiver)
  // }
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
          storage.remove(self.index)
        }
      }
    }
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
