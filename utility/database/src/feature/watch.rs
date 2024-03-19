use std::pin::Pin;
use std::task::{Context, Poll};

use futures::task::AtomicWaker;
use futures::{Stream, StreamExt};
use parking_lot::lock_api::RawRwLock;

use crate::*;

pub struct DatabaseMutationWatch {
  component_changes: RwLock<FastHashMap<TypeId, Box<dyn Any>>>,
  db: Database,
}

impl DatabaseMutationWatch {
  pub fn new(db: &Database) -> Self {
    Self {
      component_changes: Default::default(),
      db: db.clone(),
    }
  }

  pub fn watch<C: ComponentSemantic>(
    &self,
  ) -> impl ReactiveCollection<AllocIdx<C::Entity>, C::Data> {
    let type_id = TypeId::of::<C>();
    if let Some(watcher) = self.component_changes.read().get(&type_id) {
      let watcher = watcher
        .downcast_ref::<ComponentMutationWatcher<C>>()
        .unwrap();
      return watcher.forker.clone();
    }

    let (sender, receiver) = channel();
    self.db.access_ecg::<C::Entity, _>(move |e| {
      e.access_component::<C, _>(move |c| {
        c.group_watchers.on(move |change| unsafe {
          match change {
            ComponentValueChange::StartWrite => {
              sender.lock();
              false
            }
            ComponentValueChange::EndWrite => {
              sender.unlock();
              sender.is_closed()
            }
            ComponentValueChange::Write(write) => {
              sender.send(write.idx, write.change.clone());
              false
            }
          }
        })
      })
    });

    self.component_changes.write().insert(type_id, todo!());

    self.watch::<C>()
  }
}

struct ComponentMutationWatcher<C: ComponentSemantic> {
  // todo, remove watcher if all forked has been dropped
  watcher_handle: RemoveToken<IndexValueChange<C::Data>>,
  forker: RxCForker<AllocIdx<C::Entity>, C::Data>,
}

type MutationData<K, T> = FastHashMap<AllocIdx<K>, ValueChange<T>>;

fn channel<K, T>() -> (
  ComponentMutationSender<K, T>,
  ComponentMutationReceiver<K, T>,
) {
  let inner: Arc<(RwLock<MutationData<K, T>>, AtomicWaker)> = Default::default();
  let sender = ComponentMutationSender {
    inner: inner.clone(),
  };
  let receiver = ComponentMutationReceiver { inner };

  (sender, receiver)
}

struct ComponentMutationSender<K, T> {
  inner: Arc<(RwLock<MutationData<K, T>>, AtomicWaker)>,
}

impl<K, T: CValue> ComponentMutationSender<K, T> {
  unsafe fn lock(&self) {
    self.inner.0.raw().lock_exclusive()
  }

  unsafe fn unlock(&self) {
    self.inner.1.wake();
    self.inner.0.raw().unlock_exclusive()
  }
  unsafe fn send(&self, idx: AllocIdx<K>, change: ValueChange<T>) {
    let mutations = &mut *self.inner.0.data_ptr();
    if let Some(old_change) = mutations.get_mut(&idx) {
      if !old_change.merge(&change) {
        mutations.remove(&idx);
      }
    } else {
      mutations.insert(idx, change);
    }
  }
  fn is_closed(&self) -> bool {
    // self inner is shared between sender and receiver, if not shared anymore it must be
    // receiver not exist anymore, so the channel is closed.
    Arc::strong_count(&self.inner) == 1
  }
}

/// this is not likely to be triggered because component type is not get removed in any time
impl<K, T> Drop for ComponentMutationSender<K, T> {
  fn drop(&mut self) {
    self.inner.1.wake()
  }
}

pub struct ComponentMutationReceiver<K, T> {
  inner: Arc<(RwLock<MutationData<K, T>>, AtomicWaker)>,
}

impl<K, T> Stream for ComponentMutationReceiver<K, T> {
  type Item = MutationData<K, T>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    self.inner.1.register(cx.waker());
    let mut changes = self.inner.0.write();
    let changes: &mut MutationData<K, T> = &mut changes;

    let changes = std::mem::take(changes);
    if !changes.is_empty() {
      Poll::Ready(Some(changes))
      // check if the sender has been dropped
    } else if Arc::strong_count(&self.inner) == 1 {
      Poll::Ready(None)
    } else {
      Poll::Pending
    }
  }
}

struct ReactiveCollectionFromComponentMutation<K, T> {
  original: ComponentCollection<T>,
  mutation: RwLock<ComponentMutationReceiver<K, T>>,
}
impl<K: Any, T: CValue> ReactiveCollection<AllocIdx<K>, T>
  for ReactiveCollectionFromComponentMutation<K, T>
{
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<AllocIdx<K>, T> {
    match self.mutation.write().poll_next_unpin(cx) {
      Poll::Ready(Some(r)) => {
        if r.is_empty() {
          Poll::Pending
        } else {
          Poll::Ready(Box::new(r) as Box<dyn VirtualCollection<AllocIdx<K>, ValueChange<T>>>)
        }
      }
      _ => Poll::Pending,
    }
  }

  fn access(&self) -> PollCollectionCurrent<AllocIdx<K>, T> {
    // Box::new(self.original.read())
    todo!()
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    // component storage should not shrink here
  }
}
