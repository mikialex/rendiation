mod watch_group;
mod watch_linear;
mod watch_query;

use futures::FutureExt;
pub use watch_group::*;
pub use watch_linear::*;
pub use watch_query::*;

use crate::*;

pub trait DBHookCxLike: QueryHookCxLike {
  fn use_changes<C: ComponentSemantic>(
    &mut self,
  ) -> UseResult<Arc<LinearBatchChanges<u32, C::Data>>>;

  fn use_dual_query<C: ComponentSemantic>(&mut self) -> UseResult<DBDualQuery<C::Data>> {
    self.use_query_change::<C>().map(|change| DualQuery {
      view: get_db_view::<C>(),
      delta: change,
    })
  }

  fn use_query_change<C: ComponentSemantic>(&mut self) -> UseResult<DBChange<C::Data>>;
  fn use_query_set<E: EntitySemantic>(&mut self) -> UseResult<DBChange<()>>;

  #[track_caller]
  fn use_db_rev_ref_tri_view<C: ForeignKeySemantic>(&mut self) -> UseResult<RevRefForeignTriQuery> {
    let rev_many_view = self.use_db_rev_ref::<C>();
    let changes = self.use_query_change::<C>();
    // i assume this generate less code compare to join
    if self.is_spawning_stage() {
      let rev_many_view = rev_many_view.expect_spawn_stage_future();
      let changes = changes.expect_spawn_stage_ready();

      let changes = FilterMapQueryChange {
        base: changes,
        mapper: |v| v,
      }
      .into_boxed();

      UseResult::SpawnStageFuture(Box::new(rev_many_view.map(move |rev_many_view| {
        RevRefForeignTriQuery {
          base: DualQuery {
            view: get_db_view::<C>().filter_map(|v| v).into_boxed(),
            delta: changes,
          },
          rev_many_view,
        }
      })))
    } else {
      UseResult::NotInStage
    }
  }

  #[track_caller]
  fn use_db_rev_ref_typed<C: ForeignKeySemantic>(
    &mut self,
  ) -> UseResult<RevRefForeignKeyReadTyped<C>> {
    self
      .use_db_rev_ref::<C>()
      .map(|v| RevRefForeignKeyReadTyped {
        internal: v,
        phantom: PhantomData,
      })
  }

  #[track_caller]
  fn use_db_rev_ref<C: ForeignKeySemantic>(&mut self) -> UseResult<RevRefForeignKeyRead> {
    let key = match C::component_id() {
      ComponentId::TypeId(type_id) => ShareKey::TypeId(type_id),
      ComponentId::Hash(hash) => ShareKey::Hash(hash),
    };

    self.use_shared_compute_internal(
      |cx| {
        let changes = cx
          .use_query_change::<C>()
          .map(|v| v.delta_filter_map(|v| v));

        cx.use_rev_ref(changes)
      },
      key,
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
