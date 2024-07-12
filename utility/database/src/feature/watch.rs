use crate::*;

#[derive(Clone)]
pub struct DatabaseMutationWatch {
  component_changes: Arc<RwLock<FastHashMap<ComponentId, Box<dyn Any + Send + Sync>>>>,
  entity_set_changes: Arc<RwLock<FastHashMap<EntityId, Box<dyn Any + Send + Sync>>>>,
  pub(crate) db: Database,
}

impl DataBaseFeature for DatabaseMutationWatch {
  fn as_any(&self) -> &dyn Any {
    self
  }
}

impl<V: CValue> VirtualCollection<RawEntityHandle, V> for Arena<V> {
  fn iter_key_value(&self) -> impl Iterator<Item = (RawEntityHandle, V)> + '_ {
    self.iter().map(|(h, v)| {
      let raw = h.into_raw_parts();
      (
        RawEntityHandle(Handle::from_raw_parts(raw.0, raw.1)),
        v.clone(),
      )
    })
  }

  fn access(&self, key: &RawEntityHandle) -> Option<V> {
    let handle = self.get_handle(key.index() as usize).unwrap();
    self.get(handle).cloned()
  }
}

impl DatabaseMutationWatch {
  pub fn new(db: &Database) -> Self {
    Self {
      component_changes: Default::default(),
      entity_set_changes: Default::default(),
      db: db.clone(),
    }
  }

  pub fn watch_entity_set_dyn(
    &self,
    e_id: EntityId,
  ) -> impl ReactiveCollection<RawEntityHandle, ()> {
    if let Some(watcher) = self.entity_set_changes.read().get(&e_id) {
      let watcher = watcher
        .downcast_ref::<RxCForker<RawEntityHandle, ()>>()
        .unwrap();
      return watcher.clone();
    }

    let (rev, full) = self.db.access_ecg_dyn(e_id, move |e| {
      let rev = add_listen(&e.inner.entity_watchers);
      let full = e.inner.allocator.clone();
      (rev, full)
    });

    let rxc = ReactiveCollectionFromCollectiveMutation::<RawEntityHandle, ()> {
      full: Box::new(full),
      mutation: RwLock::new(rev),
    };

    self.entity_set_changes.write().insert(e_id, Box::new(rxc));

    self.watch_entity_set_dyn(e_id)
  }

  pub fn watch_untyped_key<C: ComponentSemantic>(&self) -> impl ReactiveCollection<u32, C::Data> {
    GenerationHelperView {
      inner: self.watch_dyn::<C::Data>(C::component_id(), C::Entity::entity_id()),
      phantom: PhantomData::<C::Data>,
      allocator: self
        .db
        .access_ecg::<C::Entity, _>(|e| e.inner.inner.allocator.clone()),
    }
  }

  pub fn watch<C: ComponentSemantic>(
    &self,
  ) -> impl ReactiveCollection<EntityHandle<C::Entity>, C::Data> {
    self
      .watch_dyn(C::component_id(), C::Entity::entity_id())
      .collective_key_dual_map(
        |k| unsafe { EntityHandle::<C::Entity>::from_raw(k) },
        |k| k.handle,
      )
  }

  pub fn watch_typed_foreign_key<C: ForeignKeySemantic>(
    &self,
  ) -> impl ReactiveCollection<EntityHandle<C::Entity>, Option<EntityHandle<C::ForeignEntity>>> {
    self
      .watch::<C>()
      .collective_map(|v| v.map(|v| unsafe { EntityHandle::<C::ForeignEntity>::from_raw(v) }))
  }

  pub fn watch_dyn_foreign_key(
    &self,
    component_id: ComponentId,
    entity_id: EntityId,
  ) -> impl ReactiveCollection<RawEntityHandle, ForeignKeyComponentData> {
    self.watch_dyn::<ForeignKeyComponentData>(component_id, entity_id)
  }

  pub fn watch_dyn<T: CValue>(
    &self,
    component_id: ComponentId,
    entity_id: EntityId,
  ) -> impl ReactiveCollection<RawEntityHandle, T> {
    if let Some(watcher) = self.component_changes.read().get(&component_id) {
      let watcher = watcher
        .downcast_ref::<RxCForker<RawEntityHandle, T>>()
        .unwrap();
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
      .unwrap()
    });

    let rxc = ReactiveCollectionFromCollectiveMutation {
      full: Box::new(ComponentAccess {
        ecg: self.db.access_ecg_dyn(entity_id, |ecg| ecg.clone()),
        original,
      }),
      mutation: RwLock::new(receiver),
    };

    let rxc: Box<dyn DynReactiveCollection<RawEntityHandle, T>> = Box::new(rxc);
    let rxc: RxCForker<RawEntityHandle, T> = rxc.into_static_forker();

    self
      .component_changes
      .write()
      .insert(component_id, Box::new(rxc));

    self.watch_dyn::<T>(component_id, entity_id)
  }
}

fn add_listen<T: CValue>(
  source: &EventSource<ScopedValueChange<T>>,
) -> CollectiveMutationReceiver<RawEntityHandle, T> {
  let (sender, receiver) = collective_channel::<RawEntityHandle, T>();
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

impl<T: CValue> VirtualCollectionProvider<u32, T> for ComponentAccess<T> {
  fn access(&self) -> Box<dyn DynVirtualCollection<u32, T>> {
    IterableComponentReadView::<T> {
      ecg: self.ecg.clone(),
      read_view: self.original.create_read_view(),
    }
    .into_boxed()
  }
}

impl<T: CValue> VirtualCollectionProvider<RawEntityHandle, T> for ComponentAccess<T> {
  fn access(&self) -> Box<dyn DynVirtualCollection<RawEntityHandle, T>> {
    IterableComponentReadView::<T> {
      ecg: self.ecg.clone(),
      read_view: self.original.create_read_view(),
    }
    .into_boxed()
  }
}

pub(crate) struct GenerationHelperView<T, C> {
  inner: T,
  phantom: PhantomData<C>,
  allocator: Arc<RwLock<Arena<()>>>,
}

#[derive(Clone)]
struct GenerationHelperViewAccess<T, C> {
  inner: T,
  phantom: PhantomData<C>,
  allocator: LockReadGuardHolder<Arena<()>>,
}

impl<C: CValue, T: VirtualCollection<RawEntityHandle, C> + Clone> VirtualCollection<u32, C>
  for GenerationHelperViewAccess<T, C>
{
  fn iter_key_value(&self) -> impl Iterator<Item = (u32, C)> + '_ {
    self.inner.iter_key_value().map(|(h, v)| (h.index(), v))
  }

  fn access(&self, key: &u32) -> Option<C> {
    let handle = self.allocator.get_handle(*key as usize)?;
    self.inner.access(&RawEntityHandle(handle))
  }
}

impl<C: CValue, T: ReactiveCollection<RawEntityHandle, C>> ReactiveCollection<u32, C>
  for GenerationHelperView<T, C>
{
  type Changes = impl VirtualCollection<u32, ValueChange<C>>;
  type View = impl VirtualCollection<u32, C>;
  type Task = impl Future<Output = (Self::Changes, Self::View)>;
  fn poll_changes(&self, cx: &mut Context) -> Self::Task {
    let task = self.inner.poll_changes(cx);
    let allocator = self.allocator.clone();

    async move {
      let (inner, inner_access) = task.await;

      let delta = GenerationHelperViewAccess {
        inner,
        phantom: PhantomData,
        allocator: allocator.make_read_holder(),
      };

      let access = GenerationHelperViewAccess {
        inner: inner_access,
        phantom: PhantomData,
        allocator: allocator.make_read_holder(),
      };

      (delta, access)
    }
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request)
  }
}
