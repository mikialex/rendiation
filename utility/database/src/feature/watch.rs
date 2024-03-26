use std::pin::Pin;
use std::task::{Context, Poll};

use futures::task::AtomicWaker;
use futures::{Stream, StreamExt};
use parking_lot::lock_api::RawRwLock;

use crate::*;

#[derive(Clone)]
pub struct DatabaseMutationWatch {
  component_changes: Arc<RwLock<FastHashMap<ComponentId, Box<dyn Any + Send + Sync>>>>,
  entity_set_changes: Arc<RwLock<FastHashMap<EntityId, Box<dyn Any + Send + Sync>>>>,
  db: Database,
}

impl DatabaseMutationWatch {
  pub fn new(db: &Database) -> Self {
    Self {
      component_changes: Default::default(),
      entity_set_changes: Default::default(),
      db: db.clone(),
    }
  }

  pub fn watch_entity_set<E: EntitySemantic>(&self) -> impl ReactiveCollection<u32, ()> {
    self.watch_entity_set_dyn(E::entity_id())
  }

  pub fn watch_entity_set_dyn(&self, e_id: EntityId) -> impl ReactiveCollection<u32, ()> {
    if let Some(watcher) = self.entity_set_changes.read().get(&e_id) {
      let watcher = watcher.downcast_ref::<RxCForker<u32, ()>>().unwrap();
      return watcher.clone();
    }

    let (rev, full) = self.db.access_ecg_dyn(e_id, move |e| {
      let rev = add_listen(&e.inner.entity_watchers);
      let full = e.inner.allocator.clone();
      (rev, full)
    });

    let rxc = ReactiveCollectionFromComponentMutation {
      full: Box::new(full),
      mutation: RwLock::new(rev),
    };

    self.entity_set_changes.write().insert(e_id, Box::new(rxc));

    self.watch_entity_set_dyn(e_id)
  }

  pub fn watch<C: ComponentSemantic>(&self) -> impl ReactiveCollection<u32, C::Data> {
    self.watch_dyn::<C::Data>(C::component_id(), C::Entity::entity_id())
  }

  pub fn watch_dyn_foreign_key(
    &self,
    component_id: ComponentId,
    entity_id: EntityId,
  ) -> impl ReactiveCollection<u32, ForeignKeyComponentData> {
    self.watch_dyn::<ForeignKeyComponentData>(component_id, entity_id)
  }

  pub fn watch_dyn<T: CValue>(
    &self,
    component_id: ComponentId,
    entity_id: EntityId,
  ) -> impl ReactiveCollection<u32, T> {
    if let Some(watcher) = self.component_changes.read().get(&component_id) {
      let watcher = watcher.downcast_ref::<RxCForker<u32, T>>().unwrap();
      return watcher.clone();
    }

    let (original, receiver) = self.db.access_ecg_dyn(entity_id, move |e| {
      e.access_component(component_id, move |c| {
        let event_source = c
          .inner
          .get_event_source()
          .downcast::<EventSource<ScopedValueChange<T>>>()
          .unwrap();
        let rev = add_listen(&event_source);

        let original = *c
          .inner
          .get_data()
          .downcast::<Arc<dyn ComponentStorage<T>>>()
          .unwrap();

        (original, rev)
      })
    });

    let rxc = ReactiveCollectionFromComponentMutation {
      full: Box::new(ComponentAccess {
        ecg: self.db.access_ecg_dyn(entity_id, |ecg| ecg.clone()),
        original,
      }),
      mutation: RwLock::new(receiver),
    }
    .into_static_forker();

    self
      .component_changes
      .write()
      .insert(component_id, Box::new(rxc));

    self.watch_dyn::<T>(component_id, entity_id)
  }
}

fn add_listen<T: CValue>(
  source: &EventSource<ScopedValueChange<T>>,
) -> ComponentMutationReceiver<T> {
  let (sender, receiver) = channel::<T>();
  source.on(move |change| unsafe {
    match change {
      ScopedMessage::Start => {
        sender.lock();
        false
      }
      ScopedMessage::End => {
        sender.unlock();
        sender.is_closed()
      }
      ScopedMessage::Message(write) => {
        sender.send(write.idx, write.change.clone());
        false
      }
    }
  });
  receiver
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

struct ComponentMutationReceiver<T> {
  inner: Arc<(RwLock<MutationData<T>>, AtomicWaker)>,
}

impl<T: CValue> Stream for ComponentMutationReceiver<T> {
  type Item = Box<dyn VirtualCollection<u32, ValueChange<T>>>;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
    self.inner.1.register(cx.waker());
    let mut changes = self.inner.0.write();
    let changes: &mut MutationData<T> = &mut changes;

    let changes = std::mem::take(changes);
    if !changes.is_empty() {
      Poll::Ready(Some(Box::new(changes)))
      // check if the sender has been dropped
    } else if Arc::strong_count(&self.inner) == 1 {
      Poll::Ready(None)
    } else {
      Poll::Pending
    }
  }
}

// this trait could be lift into upper stream
pub trait VirtualCollectionAccess<K, V>: Send + Sync {
  fn access(&self) -> CollectionView<K, V>;
}

impl<K: CKey, V: CValue, T: VirtualCollection<K, V> + 'static> VirtualCollectionAccess<K, V>
  for Arc<RwLock<T>>
{
  fn access(&self) -> CollectionView<K, V> {
    Box::new(self.make_read_holder())
  }
}

struct ComponentAccess<T> {
  ecg: EntityComponentGroup,
  original: Arc<dyn ComponentStorage<T>>,
}

impl<T: CValue> VirtualCollectionAccess<u32, T> for ComponentAccess<T> {
  fn access(&self) -> CollectionView<u32, T> {
    Box::new(IterableComponentReadView::<T> {
      ecg: self.ecg.clone(),
      read_view: self.original.create_read_view(),
    }) as PollCollectionCurrent<u32, T>
  }
}

struct ReactiveCollectionFromComponentMutation<T> {
  full: Box<dyn VirtualCollectionAccess<u32, T>>,
  mutation: RwLock<ComponentMutationReceiver<T>>,
}
impl<T: CValue> ReactiveCollection<u32, T> for ReactiveCollectionFromComponentMutation<T> {
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<u32, T> {
    match self.mutation.write().poll_next_unpin(cx) {
      Poll::Ready(Some(r)) => Poll::Ready(r),
      _ => Poll::Pending,
    }
  }

  fn access(&self) -> PollCollectionCurrent<u32, T> {
    self.full.access()
  }

  fn extra_request(&mut self, _request: &mut ExtraCollectionOperation) {
    // component storage should not shrink here
  }
}
