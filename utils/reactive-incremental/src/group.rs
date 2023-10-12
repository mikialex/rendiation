use std::{
  any::{Any, TypeId},
  sync::{Arc, RwLock, RwLockReadGuard, Weak},
};

use fast_hash_collection::FastHashMap;
use storage::*;

use crate::*;

#[derive(Default)]
pub struct ReactiveStoragePlane {
  storages: FastHashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

static ACTIVE_PLANE: RwLock<Option<ReactiveStoragePlane>> = RwLock::new(None);
pub fn setup_active_plane(sg: ReactiveStoragePlane) -> Option<ReactiveStoragePlane> {
  ACTIVE_PLANE.write().unwrap().replace(sg)
}

// pub fn test(scene: Scene) {
//   let attribute_bbox_stream = query_type::<AttributesMesh>().listen_by();
//   let custom_bbox_stream = query_type::<MyCustomMesh>().listen_by();

//   let mesh_stream = query_type::<MeshEnum>().listen_by();
//   let mesh_local_bbox_system = MeshLocalBBoxSystem::new(mesh_stream, attribute_bbox_stream)
//     .register_another_custom(custom_bbox_stream);

//   let mesh_model_ref_system = ..;
//   let model_scene_model_ref_system = ..;

//   let node_scene_model_ref_system = ..;

//   scheduler.register().register()
// }

struct SignalItem<T> {
  data: T,
  sub_event_handle: Option<ListHandle>,
  ref_count: u32,
  guid: u64, // weak semantics is impl by the guid compare in data access
}

pub struct IncrementalSignalGroupImpl<T: IncrementalBase> {
  data: RwLock<IndexReusedVec<SignalItem<T>>>,
  sub_watchers: RwLock<LinkListPool<EventListener<T::Delta>>>,
}

impl<T: IncrementalBase> Default for IncrementalSignalGroupImpl<T> {
  fn default() -> Self {
    Self {
      data: Default::default(),
      sub_watchers: Default::default(),
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
  inner: Weak<IncrementalSignalGroupImpl<T>>, // todo, use id
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

impl<T: IncrementalBase> IncrementalSignalPtr<T> {
  pub fn new(data: T) -> Self {
    let id = data.type_id();

    let try_read_storages = ACTIVE_PLANE.read().unwrap();
    let storages = try_read_storages
      .as_ref()
      .expect("global storage group not specified");
    if let Some(storage) = storages.storages.get(&id) {
      let storage = storage
        .downcast_ref::<IncrementalSignalStorage<T>>()
        .unwrap();
      storage.alloc(data)
    } else {
      drop(try_read_storages);
      let mut storages = ACTIVE_PLANE.write().unwrap();
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

  pub fn try_read(&self) -> Option<SignalPtrGuard<T>> {
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
      let mut sub_watcher = inner.sub_watchers.write().unwrap();
      let watcher_handle = data
        .sub_event_handle
        .get_or_insert_with(|| sub_watcher.make_list());
      sub_watcher.insert(watcher_handle, Box::new(f))
    })
  }

  pub fn off(&self, token: u32) {
    self.mutate_inner(|data, inner| {
      let mut sub_watcher = inner.sub_watchers.write().unwrap();
      sub_watcher.remove(data.sub_event_handle.as_mut().unwrap(), token);
    });
  }
  /// # Safety
  ///
  /// User should know what they're doing
  pub unsafe fn emit_manually(&self, delta: &T::Delta) {
    self.mutate_inner(|data, inner| {
      let mut sub_watcher = inner.sub_watchers.write().unwrap();
      // emit sub child
      if let Some(list) = data.sub_event_handle.as_mut() {
        sub_watcher.visit_and_remove(list, |f, _| (f(delta), true));
      }
    });
  }

  pub fn try_mutate<R>(&self, mutator: impl FnOnce(Mutating<T>) -> R) -> Option<R> {
    self.mutate_inner(|data, inner| {
      let mut sub_watcher = inner.sub_watchers.write().unwrap();

      mutator(Mutating {
        inner: &mut data.data,
        collector: &mut |delta| {
          // emit sub child
          if let Some(list) = data.sub_event_handle.as_mut() {
            sub_watcher.visit_and_remove(list, |f, _| (f(delta), true));
          }
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

  pub fn unbound_listen_by<U>(
    &self,
    mapper: impl FnMut(MaybeDeltaRef<T>, &dyn Fn(U)) + Send + Sync + 'static,
  ) -> impl Stream<Item = U>
  where
    U: Send + Sync + 'static,
  {
    self.listen_by::<U, _, _>(mapper, &DefaultUnboundChannel)
  }

  pub fn single_listen_by<U>(
    &self,
    mapper: impl FnMut(MaybeDeltaRef<T>, &dyn Fn(U)) + Send + Sync + 'static,
  ) -> impl Stream<Item = U>
  where
    U: Send + Sync + 'static,
  {
    self.listen_by::<U, _, _>(mapper, &DefaultSingleValueChannel)
  }

  pub fn listen_by<N, C, U>(
    &self,
    mut mapper: impl FnMut(MaybeDeltaRef<T>, &dyn Fn(U)) + Send + Sync + 'static,
    channel_builder: &C,
  ) -> impl Stream<Item = N>
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
    DropperAttachedStream::new(dropper, receiver)
  }

  pub fn create_drop(&self) -> impl Future<Output = ()> {
    let mut s = self.single_listen_by(no_change);

    Box::pin(async move {
      loop {
        if s.next().await.is_none() {
          break;
        }
      }
    })
  }

  pub fn defer_weak(&self) -> impl Fn(()) -> Option<Self> {
    let weak = self.downgrade();
    move |_| weak.upgrade()
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
    let mut to_remove = None; // defer the T's drop to avoid dead lock if T contains another Self
    if let Some(inner) = self.inner.upgrade() {
      let mut storage = inner.data.write().unwrap();
      let data = storage.get_mut(self.index);
      if data.guid == self.guid {
        data.ref_count -= 1;
        if data.ref_count == 0 {
          let removed = storage.remove(self.index);
          if let Some(list) = removed.sub_event_handle {
            inner.sub_watchers.write().unwrap().drop_list(list);
          }
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

impl<T: IncrementalBase> IncrementalSignalWeakPtr<T> {
  pub fn upgrade(&self) -> Option<IncrementalSignalPtr<T>> {
    if let Some(inner) = self.inner.upgrade() {
      let mut storage = inner.data.write().unwrap();
      if let Some(data) = storage.try_get_mut(self.index) {
        // maybe not valid at all
        if data.guid == self.guid {
          // event index is ok, we must check it's our data
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
  inner: RwLockReadGuard<'a, IndexReusedVec<SignalItem<T>>>,
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
  type DropFuture = impl Future<Output = ()> + Unpin;
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
