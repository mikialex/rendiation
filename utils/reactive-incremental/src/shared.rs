use std::sync::{Arc, RwLock};
use std::{
  ops::{Deref, DerefMut},
  sync::{RwLockReadGuard, RwLockWriteGuard, Weak},
};

use futures::{Future, Stream};
use incremental::IncrementalBase;
use reactive::{do_updates, ReactiveMapping};

use crate::IncrementalSignal;
use crate::*;

#[derive(Default)]
pub struct SharedIncrementalSignal<T: IncrementalBase> {
  inner: Arc<RwLock<IncrementalSignal<T>>>,

  // we keep this id on the self, to avoid unnecessary read lock.
  id: usize,
}

pub struct SceneItemWeakRef<T: IncrementalBase> {
  inner: Weak<RwLock<IncrementalSignal<T>>>,
  id: usize,
}

impl<T: IncrementalBase> SceneItemWeakRef<T> {
  pub fn upgrade(&self) -> Option<SharedIncrementalSignal<T>> {
    self
      .inner
      .upgrade()
      .map(|inner| SharedIncrementalSignal { inner, id: self.id })
  }
}

impl<T: IncrementalBase + Send + Sync> IncrementalBase for SharedIncrementalSignal<T> {
  type Delta = Self;

  fn expand(&self, mut cb: impl FnMut(Self::Delta)) {
    cb(self.clone())
  }
}

impl<T: ApplicableIncremental + Send + Sync> ApplicableIncremental for SharedIncrementalSignal<T> {
  type Error = T::Error;

  fn apply(&mut self, delta: Self::Delta) -> Result<(), Self::Error> {
    *self = delta;
    Ok(())
  }
}

impl<T: IncrementalBase> Clone for SharedIncrementalSignal<T> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
      id: self.id,
    }
  }
}

impl<T: IncrementalBase> std::hash::Hash for SharedIncrementalSignal<T> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.id.hash(state);
  }
}

impl<T: IncrementalBase> PartialEq for SharedIncrementalSignal<T> {
  fn eq(&self, other: &Self) -> bool {
    self.id == other.id
  }
}
impl<T: IncrementalBase> Eq for SharedIncrementalSignal<T> {}

impl<T: IncrementalBase> From<T> for SharedIncrementalSignal<T> {
  fn from(inner: T) -> Self {
    Self::new(inner)
  }
}

pub trait ModifySceneItemDelta<T: IncrementalBase> {
  fn apply_modify(self, target: &SharedIncrementalSignal<T>);
}

impl<T, X> ModifySceneItemDelta<T> for X
where
  T: ApplicableIncremental<Delta = X>,
{
  fn apply_modify(self, target: &SharedIncrementalSignal<T>) {
    target.mutate(|mut m| {
      m.modify(self);
    })
  }
}

impl<T: IncrementalBase> GlobalIdentified for SharedIncrementalSignal<T> {
  fn guid(&self) -> usize {
    self.id
  }
}
impl<T: IncrementalBase> AsRef<dyn GlobalIdentified> for SharedIncrementalSignal<T> {
  fn as_ref(&self) -> &(dyn GlobalIdentified + 'static) {
    self
  }
}
impl<T: IncrementalBase> AsMut<dyn GlobalIdentified> for SharedIncrementalSignal<T> {
  fn as_mut(&mut self) -> &mut (dyn GlobalIdentified + 'static) {
    self
  }
}

impl<T: IncrementalBase> SharedIncrementalSignal<T> {
  pub fn new(source: T) -> Self {
    let inner = IncrementalSignal::new(source);
    let id = inner.guid();
    let inner = Arc::new(RwLock::new(inner));
    Self { inner, id }
  }

  pub fn downgrade(&self) -> SceneItemWeakRef<T> {
    SceneItemWeakRef {
      inner: Arc::downgrade(&self.inner),
      id: self.id,
    }
  }

  pub fn defer_weak(&self) -> impl Fn(()) -> Option<Self> {
    let weak = self.downgrade();
    move |_| weak.upgrade()
  }

  pub fn pass_changes_to(
    &self,
    other: &Self,
    mut extra_mapper: impl FnMut(T::Delta) -> T::Delta + Send + Sync + 'static,
  ) where
    T: ApplicableIncremental,
  {
    let other_weak = other.downgrade();
    // here we not care the listener removal because we use weak
    self.read().delta_source.on(move |delta| {
      if let Some(other) = other_weak.upgrade() {
        other.mutate(|mut m| m.modify(extra_mapper(delta.clone())));
        false
      } else {
        true
      }
    });
  }

  pub fn trigger_change(&self, delta: &T::Delta) {
    // ignore lock poison
    let inner = self.inner.read().unwrap_or_else(|e| e.into_inner());
    let data: &T = &inner;
    let view = &DeltaView { data, delta };
    let view = unsafe { std::mem::transmute(view) };
    inner.delta_source.emit(view);
  }

  pub fn mutate<R>(&self, mutator: impl FnOnce(Mutating<T>) -> R) -> R {
    // ignore lock poison
    let mut inner = self.inner.write().unwrap_or_else(|e| e.into_inner());
    let i: &mut IncrementalSignal<T> = &mut inner;
    i.mutate(mutator)
  }
  pub fn visit<R>(&self, mut visitor: impl FnMut(&T) -> R) -> R {
    // ignore lock poison
    let inner = self.inner.read().unwrap_or_else(|e| e.into_inner());
    visitor(&inner)
  }

  pub fn read(&self) -> SceneItemRefGuard<T> {
    // ignore lock poison
    let inner = self.inner.read().unwrap_or_else(|e| e.into_inner());
    SceneItemRefGuard { inner }
  }

  pub fn write_unchecked(&self) -> SceneItemRefMutGuard<T> {
    // ignore lock poison
    let inner = self.inner.write().unwrap_or_else(|e| e.into_inner());
    SceneItemRefMutGuard { inner }
  }
}

pub struct SceneItemRefGuard<'a, T: IncrementalBase> {
  inner: RwLockReadGuard<'a, IncrementalSignal<T>>,
}

impl<'a, T: IncrementalBase> Deref for SceneItemRefGuard<'a, T> {
  type Target = IncrementalSignal<T>;

  fn deref(&self) -> &Self::Target {
    self.inner.deref()
  }
}

pub struct SceneItemRefMutGuard<'a, T: IncrementalBase> {
  inner: RwLockWriteGuard<'a, IncrementalSignal<T>>,
}

impl<'a, T: IncrementalBase> Deref for SceneItemRefMutGuard<'a, T> {
  type Target = IncrementalSignal<T>;

  fn deref(&self) -> &Self::Target {
    self.inner.deref()
  }
}

impl<'a, T: IncrementalBase> DerefMut for SceneItemRefMutGuard<'a, T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.inner.deref_mut()
  }
}

pub trait IntoSceneItemRef: Sized + IncrementalBase {
  fn into_ref(self) -> SharedIncrementalSignal<Self> {
    self.into()
  }
}

impl<T: IncrementalBase> IntoSceneItemRef for T {}

pub trait SceneItemReactiveMapping<M> {
  type ChangeStream: Stream + Unpin;
  type Ctx<'a>;

  fn build(&self, ctx: &Self::Ctx<'_>) -> (M, Self::ChangeStream);

  fn update(&self, mapped: &mut M, change: &mut Self::ChangeStream, ctx: &Self::Ctx<'_>);
}

impl<M, T> ReactiveMapping<M> for SharedIncrementalSignal<T>
where
  T: IncrementalBase + Send + Sync + 'static,
  Self: SceneItemReactiveMapping<M>,
{
  type ChangeStream = <Self as SceneItemReactiveMapping<M>>::ChangeStream;
  type DropFuture = impl Future<Output = ()> + Unpin;
  type Ctx<'a> = <Self as SceneItemReactiveMapping<M>>::Ctx<'a>;

  fn key(&self) -> usize {
    self.read().guid()
  }

  fn build(&self, ctx: &Self::Ctx<'_>) -> (M, Self::ChangeStream, Self::DropFuture) {
    let drop = self.create_drop();
    let (mapped, change) = SceneItemReactiveMapping::build(self, ctx);
    (mapped, change, drop)
  }

  fn update(&self, mapped: &mut M, change: &mut Self::ChangeStream, ctx: &Self::Ctx<'_>) {
    SceneItemReactiveMapping::update(self, mapped, change, ctx)
  }
}

pub trait SceneItemReactiveSimpleMapping<M> {
  type ChangeStream: Stream + Unpin;
  type Ctx<'a>;

  fn build(&self, ctx: &Self::Ctx<'_>) -> (M, Self::ChangeStream);
}

impl<M, T> SceneItemReactiveMapping<M> for SharedIncrementalSignal<T>
where
  T: IncrementalBase + Send + Sync + 'static,
  Self: SceneItemReactiveSimpleMapping<M>,
{
  type ChangeStream = <Self as SceneItemReactiveSimpleMapping<M>>::ChangeStream;
  type Ctx<'a> = <Self as SceneItemReactiveSimpleMapping<M>>::Ctx<'a>;

  fn build(&self, ctx: &Self::Ctx<'_>) -> (M, Self::ChangeStream) {
    SceneItemReactiveSimpleMapping::build(self, ctx)
  }

  fn update(&self, mapped: &mut M, change: &mut Self::ChangeStream, ctx: &Self::Ctx<'_>) {
    let mut pair = None;
    do_updates(change, |_| {
      pair = SceneItemReactiveMapping::build(self, ctx).into();
    });
    if let Some((new_mapped, new_change)) = pair {
      *mapped = new_mapped;
      *change = new_change;
    }
  }
}

#[macro_export]
macro_rules! with_field {
  ($ty:ty =>$field:tt) => {
    |view, send| match view {
      incremental::MaybeDeltaRef::All(value) => send(value.$field.clone()),
      incremental::MaybeDeltaRef::Delta(delta) => {
        if let incremental::DeltaOf::<$ty>::$field(field) = delta {
          send(field.clone())
        }
      }
    }
  };
}

#[macro_export]
macro_rules! with_field_expand {
  ($ty:ty =>$field:tt) => {
    |view, send| match view {
      incremental::MaybeDeltaRef::All(value) => value.$field.expand(send),
      incremental::MaybeDeltaRef::Delta(delta) => {
        if let incremental::DeltaOf::<$ty>::$field(field) = delta {
          send(field.clone())
        }
      }
    }
  };
}

#[macro_export]
macro_rules! with_field_change {
  ($ty:ty =>$field:tt) => {
    |view, send| match view {
      incremental::MaybeDeltaRef::All(value) => send(()),
      incremental::MaybeDeltaRef::Delta(delta) => {
        if let incremental::DeltaOf::<$ty>::$field(field) = delta {
          send(())
        }
      }
    }
  };
}

pub fn all_delta<T: IncrementalBase>(view: MaybeDeltaRef<T>, send: &dyn Fn(T::Delta)) {
  all_delta_with(true, Some)(view, send)
}

pub fn all_delta_no_init<T: IncrementalBase>(view: MaybeDeltaRef<T>, send: &dyn Fn(T::Delta)) {
  all_delta_with(false, Some)(view, send)
}

pub fn any_change<T: IncrementalBase>(view: MaybeDeltaRef<T>, send: &dyn Fn(())) {
  any_change_with(true)(view, send)
}

pub fn any_change_no_init<T: IncrementalBase>(view: MaybeDeltaRef<T>, send: &dyn Fn(())) {
  any_change_with(false)(view, send)
}

pub fn no_change<T: IncrementalBase>(_view: MaybeDeltaRef<T>, _send: &dyn Fn(())) {
  // do nothing at all
}

#[inline(always)]
pub fn any_change_with<T: IncrementalBase>(
  should_send_when_init: bool,
) -> impl Fn(MaybeDeltaRef<T>, &dyn Fn(())) {
  move |view, send| match view {
    MaybeDeltaRef::All(_) => {
      if should_send_when_init {
        send(())
      }
    }
    MaybeDeltaRef::Delta(_) => send(()),
  }
}

#[inline(always)]
pub fn all_delta_with<T: IncrementalBase, X>(
  should_send_when_init: bool,
  filter_map: impl Fn(T::Delta) -> Option<X>,
) -> impl Fn(MaybeDeltaRef<T>, &dyn Fn(X)) {
  move |view, send| {
    let my_send = |d| {
      if let Some(d) = filter_map(d) {
        send(d)
      }
    };
    match view {
      MaybeDeltaRef::All(value) => {
        if should_send_when_init {
          value.expand(my_send)
        }
      }
      MaybeDeltaRef::Delta(delta) => my_send(delta.clone()),
    }
  }
}

impl<T: IncrementalBase> SharedIncrementalSignal<T> {
  pub fn unbound_listen_by<U>(
    &self,
    mapper: impl FnMut(MaybeDeltaRef<T>, &dyn Fn(U)) + Send + Sync + 'static,
  ) -> impl Stream<Item = U>
  where
    U: Send + Sync + 'static,
  {
    let inner = self.read();
    inner.listen_by::<U, _, _>(mapper, &DefaultUnboundChannel)
  }

  pub fn single_listen_by<U>(
    &self,
    mapper: impl FnMut(MaybeDeltaRef<T>, &dyn Fn(U)) + Send + Sync + 'static,
  ) -> impl Stream<Item = U>
  where
    U: Send + Sync + 'static,
  {
    let inner = self.read();
    inner.listen_by::<U, _, _>(mapper, &DefaultSingleValueChannel)
  }

  pub fn listen_by<C, U>(
    &self,
    mapper: impl FnMut(MaybeDeltaRef<T>, &dyn Fn(U)) + Send + Sync + 'static,
    channel_builder: &C,
  ) -> impl Stream<Item = U>
  where
    C: ChannelLike<U, Message = U>,
    U: Send + Sync + 'static,
  {
    let inner = self.read();
    inner.listen_by::<U, C, _>(mapper, channel_builder)
  }

  pub fn create_drop(&self) -> impl Future<Output = ()> {
    let inner = self.read();
    inner.create_drop()
  }
}

#[test]
fn channel_behavior() {
  // we rely on this behavior to cleanup the sender callback
  {
    let (sender, receiver) = futures::channel::mpsc::unbounded::<usize>();
    drop(receiver);
    assert!(sender.is_closed())
  }

  // this is why we should impl custom channel
  {
    let (sender, receiver) = futures::channel::mpsc::unbounded::<usize>();
    sender.unbounded_send(999).ok();
    sender.unbounded_send(9999).ok();
    drop(sender);

    let all = futures::executor::block_on_stream(receiver).count();

    assert_eq!(all, 2)
  }

  // should wake when drop sender
  {
    use std::sync::atomic::AtomicBool;

    struct TestWaker {
      waked: Arc<AtomicBool>,
    }

    impl futures::task::ArcWake for TestWaker {
      fn wake_by_ref(arc_self: &Arc<Self>) {
        arc_self
          .waked
          .store(true, std::sync::atomic::Ordering::SeqCst);
      }
    }

    {
      let (sender, mut receiver) = futures::channel::mpsc::unbounded::<usize>();

      let test_waked = Arc::new(AtomicBool::new(false));
      let waker = Arc::new(TestWaker {
        waked: test_waked.clone(),
      });
      let waker = futures::task::waker_ref(&waker);
      let mut cx = std::task::Context::from_waker(&waker);

      // install waker
      use futures::StreamExt;
      let _ = receiver.poll_next_unpin(&mut cx);

      drop(sender);

      let waked = test_waked.load(std::sync::atomic::Ordering::SeqCst);
      assert!(waked);
    }
  }
}
