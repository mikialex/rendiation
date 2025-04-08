use futures::FutureExt;

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

#[derive(Clone)]
struct ArenaAccessProvider<T: CValue>(Arc<RwLock<Arena<T>>>);
impl<T: CValue> QueryProvider<RawEntityHandle, T> for ArenaAccessProvider<T> {
  fn access(&self) -> BoxedDynQuery<RawEntityHandle, T> {
    Box::new(ArenaAccess(self.0.make_read_holder()))
  }
}

#[derive(Clone)]
struct ArenaAccess<T: CValue>(LockReadGuardHolder<Arena<T>>);

impl<V: CValue> Query for ArenaAccess<V> {
  type Key = RawEntityHandle;
  type Value = V;
  fn iter_key_value(&self) -> impl Iterator<Item = (RawEntityHandle, V)> + '_ {
    self.0.iter().map(|(h, v)| {
      let raw = h.into_raw_parts();
      (
        RawEntityHandle(Handle::from_raw_parts(raw.0, raw.1)),
        v.clone(),
      )
    })
  }

  fn access(&self, key: &RawEntityHandle) -> Option<V> {
    let handle = self.0.get_handle(key.index() as usize).unwrap();
    self.0.get(handle).cloned()
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

  pub fn watch_entity_set<E: EntitySemantic>(
    &self,
  ) -> impl ReactiveQuery<Key = EntityHandle<E>, Value = ()> {
    self
      .watch_entity_set_dyn(E::entity_id())
      .collective_key_dual_map(|k| unsafe { EntityHandle::<E>::from_raw(k) }, |k| k.handle)
      .into_boxed_debug_large_symbol_workaround()
  }

  pub fn watch_entity_set_untyped_key<E: EntitySemantic>(
    &self,
  ) -> impl ReactiveQuery<Key = u32, Value = ()> {
    GenerationHelperView {
      inner: self.watch_entity_set_dyn(E::entity_id()),
      allocator: self
        .db
        .access_ecg::<E, _>(|e| e.inner.inner.allocator.clone()),
    }
  }

  pub fn watch_entity_set_dyn(
    &self,
    e_id: EntityId,
  ) -> impl ReactiveQuery<Key = RawEntityHandle, Value = ()> {
    if let Some(watcher) = self.entity_set_changes.read().get(&e_id) {
      let watcher = watcher
        .downcast_ref::<RQForker<RawEntityHandle, ()>>()
        .unwrap();
      return watcher.clone();
    }

    let (rev, full) = self.db.access_ecg_dyn(e_id, move |e| {
      let rev = add_listen(
        ArenaAccessProvider(e.inner.allocator.clone()),
        &e.inner.entity_watchers,
      );
      let full = e.inner.allocator.clone();
      (rev, full)
    });

    let rxc = ReactiveQueryFromCollectiveMutation::<RawEntityHandle, ()> {
      full: Box::new(ArenaAccessProvider(full)),
      mutation: RwLock::new(rev),
    };

    let rxc: BoxedDynReactiveQuery<RawEntityHandle, ()> = Box::new(rxc);
    let rxc: RQForker<RawEntityHandle, ()> = rxc.into_static_forker();

    self.entity_set_changes.write().insert(e_id, Box::new(rxc));

    self.watch_entity_set_dyn(e_id)
  }

  pub fn watch_untyped_key<C: ComponentSemantic>(
    &self,
  ) -> impl ReactiveQuery<Key = u32, Value = C::Data> {
    GenerationHelperView {
      inner: self.watch_dyn::<C::Data>(C::component_id(), C::Entity::entity_id()),
      allocator: self
        .db
        .access_ecg::<C::Entity, _>(|e| e.inner.inner.allocator.clone()),
    }
  }

  pub fn watch<C: ComponentSemantic>(
    &self,
  ) -> impl ReactiveQuery<Key = EntityHandle<C::Entity>, Value = C::Data> {
    self
      .watch_dyn(C::component_id(), C::Entity::entity_id())
      .collective_key_dual_map(
        |k| unsafe { EntityHandle::<C::Entity>::from_raw(k) },
        |k| k.handle,
      )
      .into_boxed_debug_large_symbol_workaround()
  }

  pub fn watch_typed_foreign_key<C: ForeignKeySemantic>(
    &self,
  ) -> impl ReactiveQuery<Key = EntityHandle<C::Entity>, Value = Option<EntityHandle<C::ForeignEntity>>>
  {
    self
      .watch::<C>()
      .collective_map(|v| v.map(|v| unsafe { EntityHandle::<C::ForeignEntity>::from_raw(v) }))
      .into_boxed_debug_large_symbol_workaround()
  }

  pub fn watch_dyn_foreign_key(
    &self,
    component_id: ComponentId,
    entity_id: EntityId,
  ) -> impl ReactiveQuery<Key = RawEntityHandle, Value = ForeignKeyComponentData> {
    self.watch_dyn::<ForeignKeyComponentData>(component_id, entity_id)
  }

  pub fn watch_dyn<T: CValue>(
    &self,
    component_id: ComponentId,
    entity_id: EntityId,
  ) -> impl ReactiveQuery<Key = RawEntityHandle, Value = T> {
    if let Some(watcher) = self.component_changes.read().get(&component_id) {
      let watcher = watcher
        .downcast_ref::<RQForker<RawEntityHandle, T>>()
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
        let original = *c
          .inner
          .get_data()
          .downcast::<Arc<dyn ComponentStorage<T>>>()
          .unwrap();

        let rev = add_listen(
          ComponentAccess {
            ecg: e.clone(),
            original: original.clone(),
          },
          &event_source,
        );

        (original, rev)
      })
      .unwrap()
    });

    let rxc = ReactiveQueryFromCollectiveMutation {
      full: Box::new(ComponentAccess {
        ecg: self.db.access_ecg_dyn(entity_id, |ecg| ecg.clone()),
        original,
      }),
      mutation: RwLock::new(receiver),
    };

    let rxc: BoxedDynReactiveQuery<RawEntityHandle, T> = Box::new(rxc);
    let rxc: RQForker<RawEntityHandle, T> = rxc.into_static_forker();

    self
      .component_changes
      .write()
      .insert(component_id, Box::new(rxc));

    self.watch_dyn::<T>(component_id, entity_id)
  }
}

fn add_listen<T: CValue>(
  query: impl QueryProvider<RawEntityHandle, T>,
  source: &EventSource<ScopedValueChange<T>>,
) -> CollectiveMutationReceiver<RawEntityHandle, T> {
  let (sender, receiver) = collective_channel::<RawEntityHandle, T>();
  // expand initial value while first listen.
  unsafe {
    sender.lock();
    for (idx, v) in query.access().iter_key_value() {
      sender.send(idx, ValueChange::Delta(v, None));
    }
    sender.unlock();
  }

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

impl<T: CValue> QueryProvider<u32, T> for ComponentAccess<T> {
  fn access(&self) -> BoxedDynQuery<u32, T> {
    IterableComponentReadView::<T> {
      ecg: self.ecg.clone(),
      read_view: self.original.create_read_view(),
    }
    .into_boxed()
  }
}

impl<T: CValue> QueryProvider<RawEntityHandle, T> for ComponentAccess<T> {
  fn access(&self) -> BoxedDynQuery<RawEntityHandle, T> {
    IterableComponentReadViewChecked::<T> {
      ecg: self.ecg.clone(),
      read_view: self.original.create_read_view(),
    }
    .into_boxed()
  }
}

pub trait UntypedEntityHandleExt: ReactiveQuery {
  fn untyped_entity_handle(self) -> impl ReactiveQuery<Key = u32, Value = Self::Value>;
}

impl<E, T> UntypedEntityHandleExt for T
where
  E: EntitySemantic,
  T: ReactiveQuery<Key = EntityHandle<E>>,
{
  fn untyped_entity_handle(self) -> impl ReactiveQuery<Key = u32, Value = Self::Value> {
    GenerationHelperView {
      inner: self
        .collective_key_dual_map(|k| k.handle, |k| unsafe { EntityHandle::<E>::from_raw(k) }),
      allocator: global_database().access_ecg::<E, _>(|e| e.inner.inner.allocator.clone()),
    }
  }
}

pub(crate) struct GenerationHelperView<T> {
  inner: T,
  allocator: Arc<RwLock<Arena<()>>>,
}

#[derive(Clone)]
pub struct GenerationHelperViewAccess<T> {
  inner: T,
  allocator: LockReadGuardHolder<Arena<()>>,
}

impl<T: Query<Key = RawEntityHandle> + Clone> Query for GenerationHelperViewAccess<T> {
  type Key = u32;
  type Value = T::Value;
  fn iter_key_value(&self) -> impl Iterator<Item = (u32, T::Value)> + '_ {
    self.inner.iter_key_value().map(|(h, v)| (h.index(), v))
  }

  fn access(&self, key: &u32) -> Option<T::Value> {
    let handle = self.allocator.get_handle(*key as usize)?;
    self.inner.access(&RawEntityHandle(handle))
  }
}

impl<T> QueryCompute for GenerationHelperView<T>
where
  T: QueryCompute<Key = RawEntityHandle>,
{
  type Key = u32;
  type Value = T::Value;
  type Changes = GenerationHelperViewAccess<T::Changes>;
  type View = GenerationHelperViewAccess<T::View>;

  fn resolve(&mut self, cx: &QueryResolveCtx) -> (Self::Changes, Self::View) {
    let (inner, inner_access) = self.inner.resolve(cx);

    let delta = GenerationHelperViewAccess {
      inner,
      allocator: self.allocator.make_read_holder(),
    };

    let access = GenerationHelperViewAccess {
      inner: inner_access,
      allocator: self.allocator.make_read_holder(),
    };

    (delta, access)
  }
}

impl<T> AsyncQueryCompute for GenerationHelperView<T>
where
  T: AsyncQueryCompute<Key = RawEntityHandle>,
{
  type Task = impl Future<Output = (Self::Changes, Self::View)> + 'static;

  fn create_task(&mut self, cx: &mut AsyncQueryCtx) -> Self::Task {
    let allocator = self.allocator.clone();
    let c = cx.resolve_cx().clone();
    self
      .inner
      .create_task(cx)
      .map(move |inner| GenerationHelperView { inner, allocator }.resolve(&c))
  }
}

impl<T: ReactiveQuery<Key = RawEntityHandle>> ReactiveQuery for GenerationHelperView<T> {
  type Key = u32;
  type Value = T::Value;
  type Compute = GenerationHelperView<T::Compute>;
  fn describe(&self, cx: &mut Context) -> Self::Compute {
    GenerationHelperView {
      inner: self.inner.describe(cx),
      allocator: self.allocator.clone(),
    }
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.inner.request(request)
  }
}

#[test]
fn test_watch() {
  setup_global_database(Default::default());
  setup_active_reactive_query_registry(Default::default());

  let watch = DatabaseMutationWatch::new(&global_database());
  register_global_database_feature(watch);

  declare_entity!(TestEntity);
  declare_component!(TestComponent, TestEntity, u32);

  global_database()
    .declare_entity::<TestEntity>()
    .declare_component::<TestComponent>();

  let watcher = global_watch()
    .watch::<TestComponent>()
    .debug("watch", false);

  let watcher2 = global_watch()
    .watch::<TestComponent>()
    .debug("watch2", false);

  let a = global_database()
    .entity_writer::<TestEntity>()
    .with_component_value_writer::<TestComponent>(1)
    .new_entity();
  let b = global_database()
    .entity_writer::<TestEntity>()
    .with_component_value_writer::<TestComponent>(2)
    .new_entity();

  noop_ctx!(cx);

  {
    let mut des1 = watcher.describe(cx);
    let mut des2 = watcher2.describe(cx);
    let mut des2_d = watcher2.describe(cx);
    let (d1, v1) = des1.resolve_kept();
    assert_eq!(v1.iter_key_value().count(), 2);
    assert_eq!(d1.iter_key_value().count(), 2);

    let (d2, v2) = des2.resolve_kept();
    assert_eq!(v2.iter_key_value().count(), 2);
    assert_eq!(d2.iter_key_value().count(), 2);

    let (d2, v2) = des2_d.resolve_kept();
    assert_eq!(v2.iter_key_value().count(), 2);
    assert_eq!(d2.iter_key_value().count(), 0);
  }

  {
    let watcher3 = global_watch()
      .watch::<TestComponent>()
      .debug("watch3", false)
      .into_forker();

    let (d, v) = watcher3.poll_changes_dyn(cx).resolve_kept();
    assert_eq!(v.iter_key_value().count(), 2);
    assert_eq!(d.iter_key_value().count(), 2);
  }

  {
    let (d, v) = watcher.poll_changes_dyn(cx).resolve_kept();
    assert_eq!(v.iter_key_value().count(), 2);
    assert_eq!(d.iter_key_value().count(), 0);
    let (d, v) = watcher2.poll_changes_dyn(cx).resolve_kept();
    assert_eq!(v.iter_key_value().count(), 2);
    assert_eq!(d.iter_key_value().count(), 0);
  }

  global_database()
    .entity_writer::<TestEntity>()
    .delete_entity(b);

  {
    let (d, v) = watcher.poll_changes_dyn(cx).resolve_kept();
    assert_eq!(v.iter_key_value().count(), 1);
    assert_eq!(d.iter_key_value().count(), 1);
  };

  {
    let (d, v) = watcher.poll_changes_dyn(cx).resolve_kept();
    assert_eq!(v.iter_key_value().count(), 1);
    assert_eq!(d.iter_key_value().count(), 0);
  }

  {
    let (d, v) = watcher2.poll_changes_dyn(cx).resolve_kept();
    assert_eq!(v.iter_key_value().count(), 1);
    assert_eq!(d.iter_key_value().count(), 1);
  }
  {
    let (d, v) = watcher2.poll_changes_dyn(cx).resolve_kept();
    assert_eq!(v.iter_key_value().count(), 1);
    assert_eq!(d.iter_key_value().count(), 0);
  }

  global_database()
    .entity_writer::<TestEntity>()
    .write::<TestComponent>(a, 2);

  {
    let (d, v) = watcher.poll_changes_dyn(cx).resolve_kept();
    assert_eq!(v.iter_key_value().count(), 1);
    assert_eq!(d.iter_key_value().count(), 1);
  }
}
