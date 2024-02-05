use std::sync::{Arc, RwLock};
use std::{
  ops::{Deref, DerefMut},
  sync::{RwLockReadGuard, RwLockWriteGuard, Weak},
};

use futures::{Future, Stream};
use incremental::IncrementalBase;
use reactive_stream::{do_updates, ReactiveMapping};

use crate::IncrementalSignal;
use crate::*;

pub struct SharedIncrementalSignal<T: IncrementalBase> {
  inner: Arc<RwLock<IncrementalSignal<T>>>,

  // we keep this id on the self, to avoid unnecessary read lock.
  guid: u64,
}

impl<T: IncrementalBase + Default> Default for SharedIncrementalSignal<T> {
  fn default() -> Self {
    Self::new(T::default())
  }
}

pub struct SharedIncrementalWeakSignal<T: IncrementalBase> {
  inner: Weak<RwLock<IncrementalSignal<T>>>,
  guid: u64,
}

impl<T: IncrementalBase> SharedIncrementalWeakSignal<T> {
  pub fn upgrade(&self) -> Option<SharedIncrementalSignal<T>> {
    self.inner.upgrade().map(|inner| SharedIncrementalSignal {
      inner,
      guid: self.guid,
    })
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
      guid: self.guid,
    }
  }
}

impl<T: IncrementalBase> std::hash::Hash for SharedIncrementalSignal<T> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.guid.hash(state);
  }
}

impl<T: IncrementalBase> PartialEq for SharedIncrementalSignal<T> {
  fn eq(&self, other: &Self) -> bool {
    self.guid == other.guid
  }
}
impl<T: IncrementalBase> Eq for SharedIncrementalSignal<T> {}

impl<T: IncrementalBase> From<T> for SharedIncrementalSignal<T> {
  fn from(inner: T) -> Self {
    Self::new(inner)
  }
}

pub trait SharedIncrementalSignalApplyDelta<T: IncrementalBase> {
  fn apply_modify_sh(self, target: &SharedIncrementalSignal<T>);
}

impl<T, X> SharedIncrementalSignalApplyDelta<T> for X
where
  T: ApplicableIncremental<Delta = X>,
{
  fn apply_modify_sh(self, target: &SharedIncrementalSignal<T>) {
    target.mutate(|mut m| {
      m.modify(self);
    })
  }
}

impl<T: IncrementalBase> GlobalIdentified for SharedIncrementalSignal<T> {
  fn guid(&self) -> u64 {
    self.guid
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
    Self { inner, guid: id }
  }

  pub fn downgrade(&self) -> SharedIncrementalWeakSignal<T> {
    SharedIncrementalWeakSignal {
      inner: Arc::downgrade(&self.inner),
      guid: self.guid,
    }
  }

  pub fn defer_weak(&self) -> impl Fn(()) -> Option<Self> {
    let weak = self.downgrade();
    move |_| weak.upgrade()
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

  pub fn read(&self) -> SignalRefGuard<T> {
    // ignore lock poison
    let inner = self.inner.read().unwrap_or_else(|e| e.into_inner());
    SignalRefGuard { inner }
  }
}

pub struct SignalRefGuard<'a, T: IncrementalBase> {
  inner: RwLockReadGuard<'a, IncrementalSignal<T>>,
}

impl<'a, T: IncrementalBase> Deref for SignalRefGuard<'a, T> {
  type Target = IncrementalSignal<T>;

  fn deref(&self) -> &Self::Target {
    self.inner.deref()
  }
}

pub struct SignalRefMutGuard<'a, T: IncrementalBase> {
  inner: RwLockWriteGuard<'a, IncrementalSignal<T>>,
}

impl<'a, T: IncrementalBase> Deref for SignalRefMutGuard<'a, T> {
  type Target = IncrementalSignal<T>;

  fn deref(&self) -> &Self::Target {
    self.inner.deref()
  }
}

impl<'a, T: IncrementalBase> DerefMut for SignalRefMutGuard<'a, T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    self.inner.deref_mut()
  }
}

pub trait IntoSharedIncrementalSignal: Sized + IncrementalBase {
  fn into_ref(self) -> SharedIncrementalSignal<Self> {
    self.into()
  }
}

impl<T: IncrementalBase> IntoSharedIncrementalSignal for T {}

impl<M, T> ReactiveMapping<M> for SharedIncrementalSignal<T>
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

impl<M, T> GlobalIdReactiveMapping<M> for SharedIncrementalSignal<T>
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

impl<T: IncrementalBase> IncrementalListenBy<T> for SharedIncrementalSignal<T> {
  fn listen_by<N, C, U>(
    &self,
    mapper: impl FnMut(MaybeDeltaRef<T>, &dyn Fn(U)) + Send + Sync + 'static,
    channel_builder: &mut C,
  ) -> Box<dyn Stream<Item = N> + Unpin>
  where
    U: Send + Sync + 'static,
    N: 'static,
    C: ChannelLike<U, Message = N>,
  {
    let inner = self.read();
    inner.listen_by::<N, C, U>(mapper, channel_builder)
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
