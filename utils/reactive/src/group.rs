use std::sync::{Arc, Weak};

use parking_lot::RwLockReadGuard;
use storage::*;

use crate::*;

pub struct SignalItem<T> {
  pub data: T,
  sub_event_handle: ListHandle,
  ref_count: u32,
  pub(crate) guid: u64, // weak semantics is impl by the guid compare in data access
}

pub enum StorageGroupChange<'a, T: IncrementalBase> {
  /// we are not suppose to support sub listen in group watch, so we not emit strong count ptr
  /// message.
  Create {
    index: AllocIdx<T>,
    data: &'a T,
  },
  Mutate {
    index: AllocIdx<T>,
    delta: T::Delta,
    data_before_mutate: &'a T,
  },
  Drop {
    index: AllocIdx<T>,
    data: &'a T,
  },
}

pub struct IncrementalSignalGroupImpl<T: IncrementalBase> {
  pub data: Arc<parking_lot::RwLock<IndexReusedVec<SignalItem<T>>>>,
  pub(crate) sub_watchers: parking_lot::RwLock<LinkListPool<EventListener<T::Delta>>>,
  // note, it's fake static, as long as we expose the unique lifetime to user, it's safe to user
  // side.
  pub(crate) group_watchers: EventSource<StorageGroupChange<'static, T>>,
}

impl<T: IncrementalBase> Default for IncrementalSignalGroupImpl<T> {
  fn default() -> Self {
    Self {
      data: Default::default(),
      sub_watchers: Default::default(),
      group_watchers: Default::default(),
    }
  }
}

/// data storage point
pub struct IncrementalSignalStorage<T: IncrementalBase> {
  pub inner: Arc<IncrementalSignalGroupImpl<T>>,
}

impl<T: IncrementalBase> Clone for IncrementalSignalStorage<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

impl<T: IncrementalBase> IncrementalSignalStorage<T> {
  pub fn clone_at_idx(&self, idx: AllocIdx<T>) -> Option<IncrementalSignalPtr<T>> {
    let mut i = self.inner.data.write();
    i.try_get_mut(idx.index).map(|item| {
      item.ref_count += 1;
      IncrementalSignalPtr {
        inner: Arc::downgrade(&self.inner),
        index: idx.index,
        guid: item.guid,
      }
    })
  }

  pub fn alloc(&self, data: T) -> IncrementalSignalPtr<T> {
    let mut storage = self.inner.data.write();
    let guid = alloc_global_res_id();
    let data = SignalItem {
      data,
      sub_event_handle: Default::default(),
      ref_count: 1,
      guid,
    };
    let index = storage.insert(data);
    self.inner.group_watchers.emit(&StorageGroupChange::Create {
      data: unsafe { std::mem::transmute(&storage.get(index).data) },
      index: index.into(),
    });

    IncrementalSignalPtr {
      inner: Arc::downgrade(&self.inner),
      index,
      guid,
    }
  }

  pub fn create_key_mapper<V>(
    &self,
    mapper: impl Fn(&T, u64) -> V + Send + Sync,
  ) -> impl Fn(AllocIdx<T>) -> V + Send + Sync {
    let data_holder = self.inner.clone();
    let guard = self.inner.data.read_recursive();
    let guard: RwLockReadGuard<'static, IndexReusedVec<SignalItem<T>>> =
      unsafe { std::mem::transmute(guard) };
    move |key| {
      let _ = data_holder;
      let item = guard.get(key.index);
      mapper(&item.data, item.guid)
    }
  }

  /// return should be removed from source after emitted
  pub fn on(
    &self,
    f: impl FnMut(&StorageGroupChange<T>) -> bool + Send + Sync + 'static,
  ) -> RemoveToken<group::StorageGroupChange<T>> {
    self.inner.group_watchers.on(f)
  }

  pub fn off(&self, token: RemoveToken<group::StorageGroupChange<T>>) {
    self
      .inner
      .group_watchers
      .off(unsafe { std::mem::transmute(token) })
  }
}

impl<T: IncrementalBase> Default for IncrementalSignalStorage<T> {
  fn default() -> Self {
    Self {
      inner: Default::default(),
    }
  }
}

/// RAII handle
pub struct IncrementalSignalPtr<T: IncrementalBase> {
  inner: Weak<IncrementalSignalGroupImpl<T>>,
  index: u32,
  guid: u64,
}

pub trait IntoIncrementalSignalPtr: Sized + IncrementalBase {
  fn into_ptr(self) -> IncrementalSignalPtr<Self> {
    self.into()
  }
}

impl<T: IncrementalBase> IntoIncrementalSignalPtr for T {}

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

impl<T: IncrementalBase> LinearIdentified for IncrementalSignalPtr<T> {
  fn alloc_index(&self) -> u32 {
    self.index
  }
}
impl<T: IncrementalBase> AsRef<dyn LinearIdentified> for IncrementalSignalPtr<T> {
  fn as_ref(&self) -> &(dyn LinearIdentified + 'static) {
    self
  }
}
impl<T: IncrementalBase> AsMut<dyn LinearIdentified> for IncrementalSignalPtr<T> {
  fn as_mut(&mut self) -> &mut (dyn LinearIdentified + 'static) {
    self
  }
}

impl<T: IncrementalBase> IncrementalSignalPtr<T> {
  pub fn new(data: T) -> Self {
    access_storage_of(|storage| storage.alloc(data))
  }

  fn mutate_inner<R>(
    &self,
    f: impl FnOnce(&mut SignalItem<T>, &IncrementalSignalGroupImpl<T>) -> R,
  ) -> Option<R> {
    if let Some(inner) = self.inner.upgrade() {
      let mut storage = inner.data.write();
      let data = storage.get_mut(self.index);
      if data.guid == self.guid {
        return Some(f(data, &inner));
      }
    }
    None
  }

  pub fn try_read(&self) -> Option<SignalPtrGuard<T>> {
    if let Some(inner) = self.inner.upgrade() {
      let storage = inner.data.read_recursive();
      let data = storage.get(self.index);
      if data.guid == self.guid {
        // Safety, this ref to the self holder
        let storage = unsafe { std::mem::transmute(storage) };
        return Some(SignalPtrGuard {
          _holder: inner,
          inner: storage,
          index: self.index,
          guid: self.guid,
        });
      }
    }
    None
  }
  pub fn read(&self) -> SignalPtrGuard<T> {
    self.try_read().unwrap()
  }

  /// return should be removed from source after emitted
  pub fn on(&self, f: impl FnMut(&T::Delta) -> bool + Send + Sync + 'static) -> Option<u32> {
    self.mutate_inner(|data, inner| {
      let mut sub_watcher = inner.sub_watchers.write();
      sub_watcher.insert(&mut data.sub_event_handle, Box::new(f))
    })
  }

  pub fn off(&self, token: u32) {
    self.mutate_inner(|data, inner| {
      let mut sub_watcher = inner.sub_watchers.write();
      sub_watcher.remove(&mut data.sub_event_handle, token);
    });
  }
  /// # Safety
  ///
  /// User should know what they're doing
  pub unsafe fn emit_manually(&self, delta: &T::Delta) {
    self.mutate_inner(|data, inner| {
      let mut sub_watcher = inner.sub_watchers.write();
      // emit sub child
      sub_watcher.visit_and_remove(&mut data.sub_event_handle, |f, _| (f(delta), true));
      inner.group_watchers.emit(&StorageGroupChange::Mutate {
        index: self.index.into(),
        delta: delta.clone(),
        data_before_mutate: std::mem::transmute(&data.data),
      });
    });
  }

  pub fn try_mutate<R>(&self, mutator: impl FnOnce(Mutating<T>) -> R) -> Option<R> {
    self.mutate_inner(|data, inner| {
      let mut sub_watcher = inner.sub_watchers.write();

      mutator(Mutating {
        inner: &mut data.data,
        collector: &mut |delta, raw_data| {
          // emit sub child
          sub_watcher.visit_and_remove(&mut data.sub_event_handle, |f, _| (f(delta), true));
          inner.group_watchers.emit(&StorageGroupChange::Mutate {
            index: self.index.into(),
            delta: delta.clone(),
            data_before_mutate: unsafe { std::mem::transmute(raw_data) },
          });
        },
      })
    })
  }
  pub fn mutate<R>(&self, mutator: impl FnOnce(Mutating<T>) -> R) -> R {
    self.try_mutate(mutator).unwrap()
  }

  pub fn try_visit<R>(&self, mut visitor: impl FnMut(&T) -> R) -> Option<R> {
    self.try_read().map(|r| visitor(&r))
  }
  pub fn visit<R>(&self, mut visitor: impl FnMut(&T) -> R) -> R {
    self.try_read().map(|r| visitor(&r)).unwrap()
  }

  pub fn defer_weak(&self) -> impl Fn(()) -> Option<Self> {
    let weak = self.downgrade();
    move |_| weak.upgrade()
  }
}

impl<T: IncrementalBase> IncrementalListenBy<T> for IncrementalSignalPtr<T> {
  fn listen_by<N, C, U>(
    &self,
    mut mapper: impl FnMut(MaybeDeltaRef<T>, &dyn Fn(U)) + Send + Sync + 'static,
    channel_builder: &mut C,
  ) -> Box<dyn Stream<Item = N> + Unpin>
  where
    U: Send + Sync + 'static,
    C: ChannelLike<U, Message = N>,
  {
    let (sender, receiver) = channel_builder.build();

    {
      let data = self.try_read().unwrap();
      mapper(MaybeDeltaRef::All(&data), &|mapped| {
        C::send(&sender, mapped);
      });
    }

    let remove_token = self
      .on(move |v| {
        mapper(MaybeDeltaRef::Delta(v), &|mapped| {
          C::send(&sender, mapped);
        });
        C::is_closed(&sender)
      })
      .unwrap();

    let dropper = IncrementalSignalStorageEventDropper {
      remove_token,
      weak: self.downgrade(),
    };
    Box::new(DropperAttachedStream::new(dropper, receiver))
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
      let mut storage = inner.data.write();
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
    let mut to_remove = None; // defer the T's drop to avoid dead lock if T contains another Self
    if let Some(inner) = self.inner.upgrade() {
      let mut storage = inner.data.write();
      let data = storage.get_mut(self.index);
      if data.guid == self.guid {
        data.ref_count -= 1;
        if data.ref_count == 0 {
          inner
            .sub_watchers
            .write()
            .drop_list(&mut data.sub_event_handle);
          let removed = storage.remove(self.index);
          inner.group_watchers.emit(&StorageGroupChange::Drop {
            index: self.index.into(),
            data: unsafe { std::mem::transmute(&removed.data) },
          });

          to_remove = removed.into();
        }
      }
    }
    drop(to_remove);
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

impl<T: IncrementalBase> GlobalIdentified for IncrementalSignalWeakPtr<T> {
  fn guid(&self) -> u64 {
    self.guid
  }
}
impl<T: IncrementalBase> AsRef<dyn GlobalIdentified> for IncrementalSignalWeakPtr<T> {
  fn as_ref(&self) -> &(dyn GlobalIdentified + 'static) {
    self
  }
}
impl<T: IncrementalBase> AsMut<dyn GlobalIdentified> for IncrementalSignalWeakPtr<T> {
  fn as_mut(&mut self) -> &mut (dyn GlobalIdentified + 'static) {
    self
  }
}

impl<T: IncrementalBase> LinearIdentified for IncrementalSignalWeakPtr<T> {
  fn alloc_index(&self) -> u32 {
    self.index
  }
}
impl<T: IncrementalBase> AsRef<dyn LinearIdentified> for IncrementalSignalWeakPtr<T> {
  fn as_ref(&self) -> &(dyn LinearIdentified + 'static) {
    self
  }
}
impl<T: IncrementalBase> AsMut<dyn LinearIdentified> for IncrementalSignalWeakPtr<T> {
  fn as_mut(&mut self) -> &mut (dyn LinearIdentified + 'static) {
    self
  }
}

impl<T: IncrementalBase> IncrementalSignalWeakPtr<T> {
  pub fn upgrade(&self) -> Option<IncrementalSignalPtr<T>> {
    if let Some(inner) = self.inner.upgrade() {
      let mut storage = inner.data.write();
      // maybe not valid at all (index is deallocated and not reused)
      if let Some(data) = storage.try_get_mut(self.index) {
        if data.guid == self.guid {
          // event index is ok, we must check if it's our data
          data.ref_count += 1;
          return Some(IncrementalSignalPtr {
            inner: self.inner.clone(),
            index: self.index,
            guid: self.guid,
          });
        }
      }
    }

    None
  }
}

pub struct SignalPtrGuard<'a, T: IncrementalBase> {
  _holder: Arc<IncrementalSignalGroupImpl<T>>,
  inner: parking_lot::RwLockReadGuard<'a, IndexReusedVec<SignalItem<T>>>,
  index: u32,
  guid: u64,
}

impl<'a, T: IncrementalBase> SignalPtrGuard<'a, T> {
  pub fn guid(&self) -> u64 {
    self.guid
  }
}

impl<'a, T: IncrementalBase> Deref for SignalPtrGuard<'a, T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    let inner = self.inner.deref();
    &inner.get(self.index).data
  }
}

impl<M, T> ReactiveMapping<M> for IncrementalSignalPtr<T>
where
  T: IncrementalBase + Send + Sync + 'static,
  Self: GlobalIdReactiveMapping<M>,
{
  type ChangeStream = <Self as GlobalIdReactiveMapping<M>>::ChangeStream;
  type DropFuture = impl Future<Output = ()> + Unpin + 'static;
  type Ctx<'a> = <Self as GlobalIdReactiveMapping<M>>::Ctx<'a>;

  fn key(&self) -> u64 {
    self.read().guid()
  }

  fn build(&self, ctx: &Self::Ctx<'_>) -> (M, Self::ChangeStream, Self::DropFuture) {
    let drop = self.create_drop();
    let (mapped, change) = GlobalIdReactiveMapping::build(self, ctx);
    (mapped, change, drop)
  }

  fn update(&self, mapped: &mut M, change: &mut Self::ChangeStream, ctx: &Self::Ctx<'_>) {
    GlobalIdReactiveMapping::update(self, mapped, change, ctx)
  }
}

impl<M, T> GlobalIdReactiveMapping<M> for IncrementalSignalPtr<T>
where
  T: IncrementalBase + Send + Sync + 'static,
  Self: GlobalIdReactiveSimpleMapping<M>,
{
  type ChangeStream = <Self as GlobalIdReactiveSimpleMapping<M>>::ChangeStream;
  type Ctx<'a> = <Self as GlobalIdReactiveSimpleMapping<M>>::Ctx<'a>;

  fn build(&self, ctx: &Self::Ctx<'_>) -> (M, Self::ChangeStream) {
    GlobalIdReactiveSimpleMapping::build(self, ctx)
  }

  fn update(&self, mapped: &mut M, change: &mut Self::ChangeStream, ctx: &Self::Ctx<'_>) {
    let mut pair = None;
    do_updates(change, |_| {
      pair = GlobalIdReactiveMapping::build(self, ctx).into();
    });
    if let Some((new_mapped, new_change)) = pair {
      *mapped = new_mapped;
      *change = new_change;
    }
  }
}

pub trait IncrementalSignalPtrApplyDelta<T: IncrementalBase> {
  fn apply_modify(self, target: &IncrementalSignalPtr<T>);
}

impl<T, X> IncrementalSignalPtrApplyDelta<T> for X
where
  T: ApplicableIncremental<Delta = X>,
{
  fn apply_modify(self, target: &IncrementalSignalPtr<T>) {
    target.mutate(|mut m| {
      m.modify(self);
    })
  }
}
