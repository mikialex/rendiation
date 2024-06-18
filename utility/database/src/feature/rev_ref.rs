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

    let allocator = self
      .mutation_watcher
      .db
      .access_ecg::<S::Entity, _>(|e| e.inner.inner.allocator.clone());
    let foreign_allocator = self
      .mutation_watcher
      .db
      .access_ecg::<S::ForeignEntity, _>(|e| e.inner.inner.allocator.clone());

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
#[derive(Clone)]
struct GenerationHelperMultiViewAccess<T> {
  inner: T,
  allocator: LockReadGuardHolder<Arena<()>>,
}

impl<T> VirtualCollection<u32, u32> for GenerationHelperMultiViewAccess<T>
where
  T: VirtualCollection<RawEntityHandle, RawEntityHandle> + Clone,
{
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (u32, u32)> + '_> {
    Box::new(
      self
        .inner
        .iter_key_value()
        .map(|(k, v)| (k.index(), v.index())),
    )
  }

  fn access(&self, key: &u32) -> Option<u32> {
    let handle = self.allocator.get_handle(*key as usize)?;
    let handle = RawEntityHandle(handle);
    self.inner.access(&handle).map(|v| v.index())
  }
}

impl<T> VirtualCollection<u32, ValueChange<u32>> for GenerationHelperMultiViewAccess<T>
where
  T: VirtualCollection<RawEntityHandle, ValueChange<RawEntityHandle>> + Clone,
{
  fn iter_key_value(&self) -> Box<dyn Iterator<Item = (u32, ValueChange<u32>)> + '_> {
    Box::new(
      self
        .inner
        .iter_key_value()
        .map(|(k, v)| (k.index(), v.map(|v| v.index()))),
    )
  }

  fn access(&self, key: &u32) -> Option<ValueChange<u32>> {
    let handle = self.allocator.get_handle(*key as usize)?;
    let handle = RawEntityHandle(handle);
    self.inner.access(&handle).map(|v| v.map(|v| v.index()))
  }
}

#[derive(Clone)]
struct GenerationHelperMultiViewMultiAccess<T> {
  inner: T,
  allocator: LockReadGuardHolder<Arena<()>>,
}

impl<T: VirtualMultiCollection<RawEntityHandle, RawEntityHandle>> VirtualMultiCollection<u32, u32>
  for GenerationHelperMultiViewMultiAccess<T>
{
  fn iter_key_in_multi_collection(&self) -> Box<dyn Iterator<Item = u32> + '_> {
    Box::new(self.inner.iter_key_in_multi_collection().map(|v| v.index()))
  }

  fn access_multi(&self, key: &u32) -> Option<Box<dyn Iterator<Item = u32> + '_>> {
    let handle = self.allocator.get_handle(*key as usize)?;
    let handle = RawEntityHandle(handle);
    self
      .inner
      .access_multi(&handle)
      .map(|iter| Box::new(iter.map(|v| v.index())) as Box<_>)
  }
}

impl<T: ReactiveOneToManyRelationship<RawEntityHandle, RawEntityHandle>>
  ReactiveOneToManyRelationship<u32, u32> for GenerationHelperMultiView<T>
where
  Self: ReactiveCollection<u32, u32>,
{
  fn multi_access(&self) -> Box<dyn VirtualMultiCollection<u32, u32>> {
    let inner: Box<dyn VirtualMultiCollection<RawEntityHandle, RawEntityHandle>> =
      self.inner.multi_access();
    Box::new(GenerationHelperMultiViewMultiAccess {
      inner,
      allocator: self.allocator.make_read_holder(),
    })
  }
}

impl<T> ReactiveCollection<u32, u32> for GenerationHelperMultiView<T>
where
  T: ReactiveOneToManyRelationship<RawEntityHandle, RawEntityHandle>,
{
  fn poll_changes(&self, cx: &mut Context) -> PollCollectionChanges<u32, u32> {
    self.inner.poll_changes(cx).map(|inner| {
      Box::new(GenerationHelperMultiViewAccess {
        inner,
        allocator: self.foreign_allocator.make_read_holder(),
      }) as CollectionChanges<u32, u32>
    })
  }

  fn access(&self) -> PollCollectionCurrent<u32, u32> {
    let inner: CollectionView<RawEntityHandle, RawEntityHandle> = self.inner.access();
    Box::new(GenerationHelperMultiViewAccess {
      inner,
      allocator: self.foreign_allocator.make_read_holder(),
    }) as CollectionView<u32, u32>
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request)
  }
}
