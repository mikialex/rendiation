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

  pub fn watch<C: ComponentSemantic>(&self) -> impl ReactiveCollection<u32, C::Data> {
    self.watch_dyn::<C::Data>(TypeId::of::<C>(), TypeId::of::<C::Entity>())
  }

  pub fn watch_dyn_foreign_key(
    &self,
    type_id: TypeId,
    entity_id: TypeId,
  ) -> impl ReactiveCollection<u32, u32> {
    self.watch_dyn::<u32>(type_id, entity_id)
  }

  pub fn watch_dyn<T: CValue>(
    &self,
    type_id: TypeId,
    entity_id: TypeId,
  ) -> impl ReactiveCollection<u32, T> {
    if let Some(watcher) = self.component_changes.read().get(&type_id) {
      let watcher = watcher.downcast_ref::<RxCForker<u32, T>>().unwrap();
      return watcher.clone();
    }

    let (sender, receiver) = channel::<T>();
    let original = self.db.access_ecg_dyn(entity_id, move |e| {
      e.access_component(type_id, move |c| {
        c.inner
          .get_event_source()
          .downcast::<EventSource<ComponentValueChange<T>>>()
          .unwrap()
          .on(move |change| unsafe {
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
          });
        *c.inner
          .get_data()
          .downcast::<Arc<dyn ComponentStorage<T>>>()
          .unwrap()
      })
    });

    let rxc = ReactiveCollectionFromComponentMutation {
      ecg: self.db.access_ecg_dyn(entity_id, |ecg| ecg.clone()),
      original,
      mutation: RwLock::new(receiver),
    }
    .into_static_forker();

    self
      .component_changes
      .write()
      .insert(type_id, Box::new(rxc));

    self.watch_dyn::<T>(type_id, entity_id)
  }
}

type MutationData<T> = FastHashMap<u32, ValueChange<T>>;

fn channel<T>() -> (ComponentMutationSender<T>, ComponentMutationReceiver<T>) {
  let inner: Arc<(RwLock<MutationData<T>>, AtomicWaker)> = Default::default();
  let sender = ComponentMutationSender {
    inner: inner.clone(),
  };
  let receiver = ComponentMutationReceiver { inner };

  (sender, receiver)
}

struct ComponentMutationSender<T> {
  inner: Arc<(RwLock<MutationData<T>>, AtomicWaker)>,
}

impl<T: CValue> ComponentMutationSender<T> {
  unsafe fn lock(&self) {
    self.inner.0.raw().lock_exclusive()
  }

  unsafe fn unlock(&self) {
    self.inner.1.wake();
    self.inner.0.raw().unlock_exclusive()
  }
  unsafe fn send(&self, idx: u32, change: ValueChange<T>) {
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
impl<T> Drop for ComponentMutationSender<T> {
  fn drop(&mut self) {
    self.inner.1.wake()
  }
}

pub struct ComponentMutationReceiver<T> {
  inner: Arc<(RwLock<MutationData<T>>, AtomicWaker)>,
}

impl<T> Stream for ComponentMutationReceiver<T> {
  type Item = MutationData<T>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    self.inner.1.register(cx.waker());
    let mut changes = self.inner.0.write();
    let changes: &mut MutationData<T> = &mut changes;

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

struct ReactiveCollectionFromComponentMutation<T> {
  ecg: EntityComponentGroup,
  original: Arc<dyn ComponentStorage<T>>,
  mutation: RwLock<ComponentMutationReceiver<T>>,
}
impl<T: CValue> ReactiveCollection<u32, T> for ReactiveCollectionFromComponentMutation<T> {
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<u32, T> {
    match self.mutation.write().poll_next_unpin(cx) {
      Poll::Ready(Some(r)) => {
        if r.is_empty() {
          Poll::Pending
        } else {
          Poll::Ready(Box::new(r) as Box<dyn VirtualCollection<u32, ValueChange<T>>>)
        }
      }
      _ => Poll::Pending,
    }
  }

  fn access(&self) -> PollCollectionCurrent<u32, T> {
    Box::new(IterableComponentReadView::<T> {
      ecg: self.ecg.clone(),
      read_view: self.original.create_read_view(),
    }) as PollCollectionCurrent<u32, T>
  }

  fn extra_request(&mut self, _request: &mut ExtraCollectionOperation) {
    // component storage should not shrink here
  }
}
