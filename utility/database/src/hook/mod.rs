mod changes_channel;
mod delta_channel;
mod persistence;
mod ref_counting;
mod staged_scope_watch;
mod util;

use std::hash::Hasher;

pub use changes_channel::*;
pub use delta_channel::*;
pub use persistence::*;
pub use ref_counting::*;
pub use staged_scope_watch::*;
pub use util::*;

use crate::*;

pub trait DBHookCxLike: QueryHookCxLike {
  fn use_changes<C: ComponentSemantic>(&mut self) -> UseResult<Arc<FastChangeCollector<C::Data>>> {
    self.use_changes_internal::<C::Data>(C::component_id(), C::Entity::entity_id())
  }

  #[inline(never)]
  fn use_changes_internal<T: CValue>(
    &mut self,
    c_id: ComponentId,
    e_id: EntityId,
  ) -> UseResult<Arc<FastChangeCollector<T>>> {
    let (cx, rev) = self.use_plain_state(|| {
      global_database().access_table_dyn(e_id, move |e| {
        e.access_component(c_id, move |c| {
          add_changes_listen(
            e.entity_capacity(),
            IterableComponentReadViewChecked::<T> {
              table: e.clone(),
              read_view: c.read_untyped(),
              phantom: PhantomData,
            },
            &c.data_watchers,
          )
        })
        .unwrap()
      })
    });

    let waker = cx.waker().clone();
    if let QueryHookStage::SpawnTask {
      change_collector, ..
    } = cx.stage()
    {
      let mut ctx = Context::from_waker(&waker);
      let changes = if let Poll::Ready(Some(r)) = rev.poll_impl(&mut ctx) {
        r
      } else {
        FastChangeCollector::empty()
      };

      if changes.has_change() {
        change_collector.notify_change();
        UseResult::SpawnStageReady(Arc::new(changes))
      } else {
        UseResult::NotInStage
      }
    } else {
      if rev.has_change() {
        cx.waker().wake_by_ref();
      }
      UseResult::NotInStage
    }
  }

  #[inline(never)]
  fn use_component_delta_raw<T: CValue>(
    &mut self,
    c_id: ComponentId,
    e_id: EntityId,
  ) -> UseResult<DBDelta<T>> {
    let (cx, rev) = self.use_plain_state(|| {
      global_database().access_table_dyn(e_id, move |e| {
        e.access_component(c_id, move |c| {
          add_delta_listen(
            e.entity_capacity(),
            IterableComponentReadViewChecked::<T> {
              table: e.clone(),
              read_view: c.read_untyped(),
              phantom: PhantomData,
            },
            &c.data_watchers,
          )
        })
        .unwrap()
      })
    });

    let waker = cx.waker().clone();
    if let QueryHookStage::SpawnTask {
      change_collector,
      spawner,
      ..
    } = cx.stage()
    {
      let mut ctx = Context::from_waker(&waker);
      if let Poll::Ready(Some(changes)) = rev.poll_impl(&mut ctx) {
        let f = spawner.spawn_task(|| changes.compute_query());
        let f = pin_box_in_frame(f);

        change_collector.notify_change();
        UseResult::SpawnStageFuture(f)
      } else {
        let f = pin_box_in_frame(std::future::ready(Default::default()));
        UseResult::SpawnStageFuture(f)
      }
    } else {
      if rev.has_change() {
        cx.waker().wake_by_ref();
      }
      UseResult::NotInStage
    }
  }

  fn use_query_change<C: ComponentSemantic>(
    &mut self,
  ) -> UseResult<BoxedDynQuery<RawEntityHandle, ValueChange<C::Data>>> {
    self.use_dual_query::<C>().map(|v| v.delta())
  }

  fn use_query_change_impl<T: CValue>(
    &mut self,
    c_id: ComponentId,
    e_id: EntityId,
    label: Option<&'static str>,
  ) -> UseResult<BoxedDynQuery<RawEntityHandle, ValueChange<T>>> {
    self
      .use_dual_query_impl::<T>(c_id, e_id, label)
      .map(|v| v.delta())
  }

  fn use_entity_set_delta_raw(&mut self, e_id: EntityId) -> UseResult<DBDelta<()>> {
    let (cx, rev) = self.use_plain_state(|| {
      global_database().access_table_dyn(e_id, move |e| {
        add_delta_listen(
          e.entity_capacity(),
          ArenaAccess(e.internal.allocator.make_read_holder()),
          &e.internal.entity_watchers,
        )
      })
    });

    let waker = cx.waker().clone();
    if let QueryHookStage::SpawnTask {
      change_collector,
      spawner,
      ..
    } = cx.stage()
    {
      let mut ctx = Context::from_waker(&waker);
      if let Poll::Ready(Some(changes)) = rev.poll_impl(&mut ctx) {
        let f = spawner.spawn_task(|| changes.compute_query());
        let f = pin_box_in_frame(f);

        change_collector.notify_change();
        UseResult::SpawnStageFuture(f)
      } else {
        let f = pin_box_in_frame(std::future::ready(Default::default()));
        UseResult::SpawnStageFuture(f)
      }
    } else {
      if rev.has_change() {
        cx.waker().wake_by_ref();
      }
      UseResult::NotInStage
    }
  }

  fn use_query_set<E: EntitySemantic>(
    &mut self,
  ) -> UseResult<BoxedDynQuery<RawEntityHandle, ValueChange<()>>> {
    self.use_dual_query_set::<E>().map(|v| v.delta())
  }

  fn use_dual_query<C: ComponentSemantic>(
    &mut self,
  ) -> UseResult<BoxedDynDualQuery<RawEntityHandle, C::Data>> {
    self.use_dual_query_impl::<C::Data>(
      C::component_id(),
      C::Entity::entity_id(),
      Some(C::unique_name()),
    )
  }

  fn use_dual_query_impl<T: CValue>(
    &mut self,
    c_id: ComponentId,
    e_id: EntityId,
    label: Option<&'static str>,
  ) -> UseResult<BoxedDynDualQuery<RawEntityHandle, T>> {
    #[derive(Clone, Copy)]
    struct DBDualQueryProvider<T>(PhantomData<T>, ComponentId, EntityId, Option<&'static str>);

    impl<T: CValue, Cx: DBHookCxLike> SharedResultProvider<Cx> for DBDualQueryProvider<T> {
      type Result = DBDualQuery<T>;
      fn compute_share_key(&self) -> ShareKey {
        match self.1 {
          ComponentId::TypeId(type_id) => ShareKey::TypeId(type_id),
          ComponentId::Hash(hash) => ShareKey::Hash(hash),
        }
      }

      fn debug_label(&self) -> &str {
        if let Some(label) = self.3 {
          label
        } else {
          std::any::type_name::<Self>()
        }
      }

      fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
        let Self(_, c_id, e_id, _) = *self;
        cx.use_component_delta_raw::<T>(c_id, e_id)
          .map(move |change| DualQuery {
            view: get_db_view_internal::<T>(e_id, c_id),
            delta: change,
          })
      }
    }

    self.use_shared_dual_query(DBDualQueryProvider::<T>(PhantomData, c_id, e_id, label))
  }

  fn use_dual_query_set<E: EntitySemantic>(
    &mut self,
  ) -> UseResult<BoxedDynDualQuery<RawEntityHandle, ()>> {
    self.use_dual_query_set_raw(E::entity_id())
  }

  fn use_dual_query_set_raw(
    &mut self,
    entity_id: EntityId,
  ) -> UseResult<BoxedDynDualQuery<RawEntityHandle, ()>> {
    struct DBDualQuerySetProvider(EntityId);

    impl<Cx: DBHookCxLike> SharedResultProvider<Cx> for DBDualQuerySetProvider {
      type Result = DBSetDualQuery;
      fn compute_share_key(&self) -> ShareKey {
        match self.0 {
          EntityId(type_id) => ShareKey::TypeId(type_id),
        }
      }

      fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
        let e_id = self.0;
        cx.use_entity_set_delta_raw(self.0)
          .map(move |change| DualQuery {
            view: global_database().access_table_dyn(e_id, |t| {
              ArenaAccess(t.internal.allocator.make_read_holder()).into_boxed()
            }),
            delta: change,
          })
      }
    }

    self.use_shared_dual_query(DBDualQuerySetProvider(entity_id))
  }

  fn use_db_rev_ref_tri_view<C: ForeignKeySemantic>(&mut self) -> UseResult<RevRefForeignTriQuery> {
    self.use_db_rev_ref_tri_view_impl(C::component_id(), C::Entity::entity_id())
  }

  #[inline(never)]
  fn use_db_rev_ref_tri_view_impl(
    &mut self,
    c_id: ComponentId,
    e_id: EntityId,
  ) -> UseResult<RevRefForeignTriQuery> {
    let rev_many_view = self.use_db_rev_ref_internal(c_id, e_id);
    let changes = self.use_query_change_impl(c_id, e_id, None);

    rev_many_view
      .join(changes)
      .map(move |(rev_many_view, changes)| RevRefForeignTriQuery {
        base: DualQuery {
          view: get_db_view_internal(e_id, c_id)
            .filter_map(|v| v)
            .into_boxed(),
          delta: FilterMapQueryChange {
            base: changes,
            mapper: |v| v,
          }
          .into_boxed(),
        },
        rev_many_view,
      })
  }

  fn use_db_rev_ref_typed<C: ForeignKeySemantic>(
    &mut self,
  ) -> UseResult<RevRefForeignKeyReadTyped<C>> {
    self
      .use_db_rev_ref_internal(C::component_id(), C::Entity::entity_id())
      .map(|v| RevRefForeignKeyReadTyped {
        internal: v,
        phantom: PhantomData,
      })
  }

  fn use_db_rev_ref<C: ForeignKeySemantic>(&mut self) -> UseResult<RevRefForeignKeyRead> {
    self.use_db_rev_ref_internal(C::component_id(), C::Entity::entity_id())
  }

  #[inline(never)]
  fn use_db_rev_ref_internal(
    &mut self,
    c_id: ComponentId,
    e_id: EntityId,
  ) -> UseResult<RevRefForeignKeyRead> {
    struct Marker;
    let mut hasher = FastHasher::default();
    c_id.hash(&mut hasher);
    TypeId::of::<Marker>().hash(&mut hasher);
    let key = ShareKey::Hash(hasher.finish());

    let consumer_id = self.use_shared_consumer(key);
    let label = "db rev_ref";
    self.use_shared_compute_internal(
      &|cx| {
        let changes = cx
          .use_query_change_impl::<Option<RawEntityHandle>>(c_id, e_id, None)
          .map(|v| v.delta_filter_map(|v| v));

        cx.use_rev_ref(changes)
      },
      key,
      label,
      consumer_id,
    )
  }
}

pub type RevRefForeignKeyRead = RevRefContainerRead<RawEntityHandle, RawEntityHandle>;
pub type RevRefForeignTriQuery = TriQuery<
  BoxedDynQuery<RawEntityHandle, RawEntityHandle>,
  BoxedDynQuery<RawEntityHandle, ValueChange<RawEntityHandle>>,
  RevRefForeignKeyRead,
>;

/// we can also using composer to implement this, like [get_db_view_typed_foreign]
pub struct RevRefForeignKeyReadTyped<C> {
  pub internal: RevRefForeignKeyRead,
  pub phantom: PhantomData<C>,
}

impl<C> Clone for RevRefForeignKeyReadTyped<C> {
  fn clone(&self) -> Self {
    Self {
      internal: self.internal.clone(),
      phantom: self.phantom,
    }
  }
}

impl<C: ForeignKeySemantic> MultiQuery for RevRefForeignKeyReadTyped<C> {
  type Key = EntityHandle<C::ForeignEntity>;
  type Value = EntityHandle<C::Entity>;

  fn iter_keys(&self) -> impl Iterator<Item = Self::Key> + '_ {
    self
      .internal
      .iter_keys()
      .map(|k| unsafe { EntityHandle::<C::ForeignEntity>::from_raw(k) })
  }

  fn access_multi(&self, key: &Self::Key) -> Option<impl Iterator<Item = Self::Value> + '_> {
    self
      .internal
      .access_multi(key.raw_handle_ref())
      .map(|iter| iter.map(|v| unsafe { EntityHandle::<C::Entity>::from_raw(v) }))
  }
}

pub trait ForeignKeyLikeChangesExt: DataChanges<Value = Option<RawEntityHandle>> {
  fn map_some_u32_index(self) -> impl DataChanges<Key = Self::Key, Value = u32> {
    self.collective_filter_map(|id| id.map(|v| v.index()))
  }
  fn map_u32_index_or_u32_max(self) -> impl DataChanges<Key = Self::Key, Value = u32> {
    self.collective_map(|id| id.map(|v| v.index()).unwrap_or(u32::MAX))
  }
}
impl<T: DataChanges<Value = Option<RawEntityHandle>>> ForeignKeyLikeChangesExt for T {}

pub trait DBUseResultExt<Q>: Sized {
  fn map_raw_handle_or_u32_max_changes(
    self,
  ) -> UseResult<impl DataChanges<Key = RawEntityHandle, Value = u32>>
  where
    Q: DualQueryLike<Key = RawEntityHandle, Value = Option<RawEntityHandle>> + 'static;
}

impl<Q: DualQueryLike> DBUseResultExt<Q> for UseResult<Q> {
  fn map_raw_handle_or_u32_max_changes(
    self,
  ) -> UseResult<impl DataChanges<Key = RawEntityHandle, Value = u32>>
  where
    Q: DualQueryLike<Key = RawEntityHandle, Value = Option<RawEntityHandle>> + 'static,
  {
    self.map(|v| {
      v.view_delta()
        .1
        .delta_map_value(map_raw_handle_or_u32_max)
        .into_change()
    })
  }
}
