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

    let rxc = ReactiveCollectionFromCollectiveMutation {
      full: Box::new(full),
      mutation: RwLock::new(rev),
    };

    self.entity_set_changes.write().insert(e_id, Box::new(rxc));

    self.watch_entity_set_dyn(e_id)
  }

  pub fn watch<C: ComponentSemantic>(&self) -> impl ReactiveCollection<u32, C::Data> {
    self.watch_dyn::<C::Data>(C::component_id(), C::Entity::entity_id())
  }
  pub fn watch_typed_key<C: ComponentSemantic>(
    &self,
  ) -> impl ReactiveCollection<AllocIdx<C::Entity>, C::Data> {
    self
      .watch::<C>()
      .collective_key_convert(AllocIdx::from, AllocIdx::into_alloc_index)
  }

  pub fn watch_typed_foreign_key<C: ForeignKeySemantic>(
    &self,
  ) -> impl ReactiveCollection<AllocIdx<C::Entity>, Option<EntityHandle<C::ForeignEntity>>> {
    todo!();
    // self
    //   .watch::<C>()
    //   .collective_key_convert(|k| AllocIdx::from(k), |k| k.index)
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

    let rxc = ReactiveCollectionFromCollectiveMutation {
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
) -> CollectiveMutationReceiver<T> {
  let (sender, receiver) = collective_channel::<T>();
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
