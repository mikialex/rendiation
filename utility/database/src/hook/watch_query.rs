use crate::*;

pub struct DBQueryChangeWatchGroup {
  internal: DBChangeWatchGroup<ComponentId>,
}

pub type DBView<V> = IterableComponentReadViewChecked<V>;
pub type DBDelta<V> = Arc<FastHashMap<RawEntityHandle, ValueChange<V>>>;
pub type DBDualQuery<V> = DualQuery<DBView<V>, DBDelta<V>>;
pub type DBSetDualQuery = DualQuery<BoxedDynQuery<RawEntityHandle, ()>, DBDelta<()>>;

impl DBQueryChangeWatchGroup {
  pub fn new(db: &Database) -> Self {
    Self {
      internal: DBChangeWatchGroup::new(db),
    }
  }

  pub fn clear_changes(&mut self) {
    self.internal.clear_changes();
  }

  pub fn allocate_next_consumer_id(&mut self) -> u32 {
    self.internal.allocate_next_consumer_id()
  }

  pub fn notify_consumer_dropped(&mut self, component_id: ComponentId, consumer_id: u32) {
    self
      .internal
      .notify_consumer_dropped(component_id, consumer_id);
  }

  pub fn get_buffered_changes<C: ComponentSemantic>(&mut self, id: u32) -> DBDelta<C::Data> {
    self.get_buffered_changes_internal(id, C::Entity::entity_id(), C::component_id())
  }

  #[inline(never)] // remove the variant of component semantic to reduce the binary bloat
  fn get_buffered_changes_internal<T: CValue>(
    &mut self,
    id: u32,
    e_id: EntityId,
    c_id: ComponentId,
  ) -> DBDelta<T> {
    let rev = self.internal.producers.entry(c_id).or_insert_with(|| {
      let rev = self.internal.db.access_ecg_dyn(e_id, move |e| {
        e.access_component(c_id, move |c| {
          add_listen(
            ComponentAccess {
              ecg: e.clone(),
              original: c.clone(),
              phantom: PhantomData::<T>,
            },
            &c.data_watchers,
          )
        })
        .unwrap()
      });
      Box::new(rev)
    });

    let rev = rev
      .downcast_mut::<CollectiveMutationReceiver<RawEntityHandle, T>>()
      .unwrap();

    let consumer_ids = self.internal.consumers.entry(c_id).or_default();

    let changes = self
      .internal
      .current_results
      .entry(c_id)
      .or_insert_with(|| {
        noop_ctx!(cx);

        let changes = if let Poll::Ready(Some(changes)) = rev.poll_impl(cx) {
          changes
        } else {
          Default::default()
        };
        Box::new(Arc::new(changes))
      });

    if consumer_ids.contains(&id) {
      let changes = changes.downcast_ref::<DBDelta<T>>().unwrap().clone();

      changes
    } else {
      consumer_ids.insert(id);
      // for any new watch created we emit full table

      let full_view = get_db_view_internal::<T>(e_id, c_id);

      let full_view_as_delta = full_view
        .iter_key_value()
        .map(|(k, v)| (k, ValueChange::Delta(v, None)))
        .collect::<FastHashMap<_, _>>(); // todo avoid collect

      Arc::new(full_view_as_delta)
    }
  }
}

pub struct DBQueryEntitySetWatchGroup {
  internal: DBChangeWatchGroup<EntityId>,
}

impl DBQueryEntitySetWatchGroup {
  pub fn new(db: &Database) -> Self {
    Self {
      internal: DBChangeWatchGroup::new(db),
    }
  }

  pub fn clear_changes(&mut self) {
    self.internal.clear_changes();
  }

  pub fn allocate_next_consumer_id(&mut self) -> u32 {
    self.internal.allocate_next_consumer_id()
  }

  pub fn notify_consumer_dropped(&mut self, e_id: EntityId, consumer_id: u32) {
    self.internal.notify_consumer_dropped(e_id, consumer_id);
  }

  pub fn get_buffered_changes<E: EntitySemantic>(&mut self, id: u32) -> DBDelta<()> {
    self.get_buffered_changes_internal(id, E::entity_id())
  }

  #[inline(never)] // remove the variant of component semantic to reduce the binary bloat
  fn get_buffered_changes_internal(&mut self, id: u32, e_id: EntityId) -> DBDelta<()> {
    let rev = self.internal.producers.entry(e_id).or_insert_with(|| {
      let rev = self.internal.db.access_ecg_dyn(e_id, move |e| {
        add_listen(
          ArenaAccessProvider(e.inner.allocator.clone()),
          &e.inner.entity_watchers,
        )
      });

      Box::new(rev)
    });

    let rev = rev
      .downcast_mut::<CollectiveMutationReceiver<RawEntityHandle, ()>>()
      .unwrap();

    let consumer_ids = self.internal.consumers.entry(e_id).or_default();

    let changes = self
      .internal
      .current_results
      .entry(e_id)
      .or_insert_with(|| {
        noop_ctx!(cx);

        let changes = if let Poll::Ready(Some(changes)) = rev.poll_impl(cx) {
          changes
        } else {
          Default::default()
        };
        Box::new(Arc::new(changes))
      });

    if consumer_ids.contains(&id) {
      let changes = changes.downcast_ref::<DBDelta<()>>().unwrap().clone();

      changes
    } else {
      consumer_ids.insert(id);
      // for any new watch created we emit full table

      let full_view = self
        .internal
        .db
        .access_ecg_dyn(e_id, move |e| e.inner.allocator.clone());
      let full_view = Box::new(ArenaAccessProvider(full_view));

      let full_view_as_delta = full_view
        .access()
        .iter_key_value()
        .map(|(k, v)| (k, ValueChange::Delta(v, None)))
        .collect::<FastHashMap<_, _>>(); // todo avoid collect

      Arc::new(full_view_as_delta)
    }
  }
}

fn get_db_view_internal<T>(e_id: EntityId, c_id: ComponentId) -> DBView<T> {
  global_database()
    .access_ecg_dyn(e_id, |ecg| {
      ecg.access_component(c_id, |c| IterableComponentReadViewChecked {
        ecg: ecg.clone(),
        read_view: c.read_untyped(),
        phantom: PhantomData,
      })
    })
    .unwrap()
}

pub fn get_db_entity_set_view<E: EntitySemantic>() -> BoxedDynQuery<RawEntityHandle, ()> {
  global_database().access_ecg_dyn(E::entity_id(), |ecg| {
    ArenaAccessProvider(ecg.inner.allocator.clone()).access()
  })
}

pub fn get_db_view<C: ComponentSemantic>() -> DBView<C::Data> {
  get_db_view_internal(C::Entity::entity_id(), C::component_id())
}

pub fn get_db_view_typed<C: ComponentSemantic>(
) -> impl Query<Key = EntityHandle<C::Entity>, Value = C::Data> {
  get_db_view_internal(C::Entity::entity_id(), C::component_id()).mark_entity_type::<C::Entity>()
}

pub fn get_db_view_typed_foreign<C: ForeignKeySemantic>(
) -> impl Query<Key = EntityHandle<C::Entity>, Value = EntityHandle<C::ForeignEntity>> {
  get_db_view_typed::<C>()
    .filter_map(|v| v.map(|v| unsafe { EntityHandle::<C::ForeignEntity>::from_raw(v) }))
}

pub trait RawEntityHandleQueryExt: Query<Key = RawEntityHandle> {
  fn mark_entity_type<E: EntitySemantic>(
    self,
  ) -> impl Query<Key = EntityHandle<E>, Value = Self::Value> {
    self.key_dual_map(|k| unsafe { EntityHandle::<E>::from_raw(k) }, |k| k.handle)
  }
}

impl<T> RawEntityHandleQueryExt for T where T: Query<Key = RawEntityHandle> {}
