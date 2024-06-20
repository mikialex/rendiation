use crate::*;

#[derive(Clone)]
pub struct DatabaseEntityReverseReference {
  mutation_watcher: DatabaseMutationWatch,
  entity_rev_refs: Arc<RwLock<StreamMap<ComponentId, Box<dyn Any + Send + Sync>>>>,
}

impl DatabaseEntityReverseReference {
  pub fn new(mutation_watcher: DatabaseMutationWatch) -> Self {
    Self {
      mutation_watcher,
      entity_rev_refs: Default::default(),
    }
  }

  pub fn watch_inv_ref<S: ForeignKeySemantic>(
    &self,
  ) -> impl ReactiveOneToManyRelationship<EntityHandle<S::ForeignEntity>, EntityHandle<S::Entity>>
  {
    self
      .watch_inv_ref_dyn(S::component_id(), S::Entity::entity_id())
      .map_value(|v| unsafe { EntityHandle::from_raw(*v) })
      .dual_map_key(|k| unsafe { EntityHandle::from_raw(*k) }, |k| k.handle)
  }

  pub fn watch_inv_ref_untyped<S: ForeignKeySemantic>(
    &self,
  ) -> Box<dyn ReactiveOneToManyRelationship<u32, u32>> {
    let inner = self.watch_inv_ref_dyn(S::component_id(), S::Entity::entity_id());

    let db = &self.mutation_watcher.db;
    let allocator = db.access_ecg::<S::Entity, _>(|e| e.inner.inner.allocator.clone());
    let foreign_allocator =
      db.access_ecg::<S::ForeignEntity, _>(|e| e.inner.inner.allocator.clone());

    Box::new(GenerationHelperMultiView {
      inner,
      allocator,
      foreign_allocator,
    })
  }

  pub fn watch_inv_ref_dyn(
    &self,
    semantic_id: ComponentId,
    entity_id: EntityId,
  ) -> Box<dyn ReactiveOneToManyRelationship<RawEntityHandle, RawEntityHandle>> {
    if let Some(refs) = self.entity_rev_refs.read().get(&semantic_id) {
      return Box::new(
        refs
          .downcast_ref::<OneManyRelationForker<RawEntityHandle, RawEntityHandle>>()
          .unwrap()
          .clone(),
      );
    }

    let watcher = self
      .mutation_watcher
      .watch_dyn_foreign_key(semantic_id, entity_id)
      .collective_filter_map(|v| v)
      .into_boxed()
      .into_one_to_many_by_hash()
      .into_static_forker();

    self
      .entity_rev_refs
      .write()
      .insert(semantic_id, Box::new(watcher));

    self.watch_inv_ref_dyn(semantic_id, entity_id)
  }
}

pub(crate) struct GenerationHelperMultiView<T> {
  inner: T,
  allocator: Arc<RwLock<Arena<()>>>,
  foreign_allocator: Arc<RwLock<Arena<()>>>,
}

impl<T: ReactiveOneToManyRelationship<RawEntityHandle, RawEntityHandle>>
  ReactiveOneToManyRelationship<u32, u32> for GenerationHelperMultiView<T>
where
  Self: ReactiveCollection<u32, u32>,
{
  fn multi_access(&self) -> Box<dyn VirtualMultiCollection<u32, u32>> {
    let allocator = self.allocator.make_read_holder();

    self
      .inner
      .multi_access()
      .map(|v| v.index())
      .key_dual_map_partial(
        |k| k.index(),
        move |k| RawEntityHandle(allocator.get_handle(k as usize)?).into(),
      )
      .into_boxed()
  }
}

impl<T> ReactiveCollection<u32, u32> for GenerationHelperMultiView<T>
where
  T: ReactiveOneToManyRelationship<RawEntityHandle, RawEntityHandle>,
{
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<u32, u32> {
    self.inner.poll_changes(cx).map(|inner| {
      let allocator = self.foreign_allocator.make_read_holder();
      inner
        .key_dual_map_partial(
          |k| k.index(),
          move |k| RawEntityHandle(allocator.get_handle(k as usize)?).into(),
        )
        .map(|_, v| v.map(|v| v.index()))
        .into_boxed()
    })
  }

  fn access(&self) -> PollCollectionCurrent<u32, u32> {
    let allocator = self.foreign_allocator.make_read_holder();
    self
      .inner
      .access()
      .key_dual_map_partial(
        |k| k.index(),
        move |k| RawEntityHandle(allocator.get_handle(k as usize)?).into(),
      )
      .map(|_, v| v.index())
      .into_boxed()
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request)
  }
}
