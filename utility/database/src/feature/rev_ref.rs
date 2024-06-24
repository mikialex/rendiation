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
  ) -> impl ReactiveOneToManyRelation<EntityHandle<S::ForeignEntity>, EntityHandle<S::Entity>> {
    self
      .watch_inv_ref_dyn(S::component_id(), S::Entity::entity_id())
      .map_value(|v| unsafe { EntityHandle::from_raw(*v) })
      .dual_map_key(|k| unsafe { EntityHandle::from_raw(*k) }, |k| k.handle)
  }

  pub fn watch_inv_ref_untyped<S: ForeignKeySemantic>(
    &self,
  ) -> Box<dyn DynReactiveOneToManyRelation<u32, u32>> {
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
  ) -> Box<dyn DynReactiveOneToManyRelation<RawEntityHandle, RawEntityHandle>> {
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

impl<T> ReactiveCollection<u32, u32> for GenerationHelperMultiView<T>
where
  T: ReactiveOneToManyRelation<RawEntityHandle, RawEntityHandle>,
{
  type Changes = impl VirtualCollection<u32, ValueChange<u32>>;
  type View = impl VirtualCollection<u32, u32> + VirtualMultiCollection<u32, u32>;
  fn poll_changes(&self, cx: &mut Context) -> (Self::Changes, Self::View) {
    let (d, v) = self.inner.poll_changes(cx);

    let allocator = self.foreign_allocator.make_read_holder();
    let d = d
      .key_dual_map_partial(
        |k| k.index(),
        move |k| RawEntityHandle(allocator.get_handle(k as usize)?).into(),
      )
      .map(|_, v| v.map(|v| v.index()))
      .into_boxed();

    let allocator = self.foreign_allocator.make_read_holder();

    // todo, improve trait builder method
    let f_v = KeyDualMapCollection {
      phantom: PhantomData,
      base: v.clone(),
      f1: |k: RawEntityHandle| k.index(),
      f2: move |k| RawEntityHandle(allocator.get_handle(k as usize)?).into(),
    };

    let f_v = VirtualCollectionExt::map(f_v, |_: &u32, v: RawEntityHandle| v.index());

    let allocator = self.allocator.make_read_holder();
    let inv = KeyDualMapCollection {
      phantom: PhantomData,
      base: v,
      f1: |k: RawEntityHandle| k.index(),
      f2: move |k| RawEntityHandle(allocator.get_handle(k as usize)?).into(),
    };
    let inv = VirtualMultiCollectionExt::map(inv, |_: &u32, v: RawEntityHandle| v.index());

    let v = OneManyRelationDualAccess {
      many_access_one: Box::new(f_v),
      one_access_many: Box::new(inv),
    };

    (d, v)
  }

  fn extra_request(&mut self, request: &mut ExtraCollectionOperation) {
    self.inner.extra_request(request)
  }
}
