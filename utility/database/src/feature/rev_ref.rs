use crate::*;

#[derive(Clone)]
pub struct DatabaseEntityReverseReference {
  mutation_watcher: DatabaseMutationWatch,
  entity_rev_refs: Arc<RwLock<FastHashMap<ComponentId, Box<dyn Any + Send + Sync>>>>,
}

impl DataBaseFeature for DatabaseEntityReverseReference {
  fn as_any(&self) -> &dyn Any {
    self
  }
}

pub type RevRefOfForeignKeyWatch<S> = BoxedDynReactiveOneToManyRelation<
  EntityHandle<<S as ForeignKeySemantic>::ForeignEntity>,
  EntityHandle<<S as EntityAssociateSemantic>::Entity>,
>;

pub type RevRefOfForeignKey<S> = BoxedDynMultiQuery<
  EntityHandle<<S as ForeignKeySemantic>::ForeignEntity>,
  EntityHandle<<S as EntityAssociateSemantic>::Entity>,
>;

impl DatabaseEntityReverseReference {
  pub fn new(mutation_watcher: DatabaseMutationWatch) -> Self {
    Self {
      mutation_watcher,
      entity_rev_refs: Default::default(),
    }
  }

  pub fn update_and_read<S: ForeignKeySemantic>(&self) -> RevRefOfForeignKey<S> {
    let view = self
      .entity_rev_refs
      .read()
      .get(&S::component_id())
      .unwrap()
      .downcast_ref::<OneManyRelationForker<RawEntityHandle, RawEntityHandle>>()
      .unwrap()
      .update_and_read();

    view
      .multi_key_dual_map(|k| unsafe { EntityHandle::from_raw(k) }, |k| k.handle)
      .multi_map(|v| unsafe { EntityHandle::from_raw(v) })
      .into_boxed_multi()
  }

  pub fn watch_inv_ref<S: ForeignKeySemantic>(
    &self,
  ) -> impl ReactiveOneToManyRelation<One = EntityHandle<S::ForeignEntity>, Many = EntityHandle<S::Entity>>
  {
    self
      .watch_inv_ref_dyn(S::component_id(), S::Entity::entity_id())
      .collective_map_key_one_many(|v| unsafe { EntityHandle::from_raw(v) }, |k| k.handle)
      .collective_dual_map_one_many(|k| unsafe { EntityHandle::from_raw(k) }, |k| k.handle)
      .into_boxed_many_one_debug_large_symbol_workaround()
  }

  pub fn watch_inv_ref_untyped<S: ForeignKeySemantic>(
    &self,
  ) -> BoxedDynReactiveOneToManyRelation<u32, u32> {
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
  ) -> BoxedDynReactiveOneToManyRelation<RawEntityHandle, RawEntityHandle> {
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
      .into_one_to_many_by_hash();

    let watcher: OneManyRelationForker<RawEntityHandle, RawEntityHandle> = (Box::new(watcher)
      as BoxedDynReactiveOneToManyRelation<RawEntityHandle, RawEntityHandle>)
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

impl<T> ReactiveQuery for GenerationHelperMultiView<T>
where
  T: ReactiveOneToManyRelation<One = RawEntityHandle, Many = RawEntityHandle>,
{
  type Key = u32;
  type Value = u32;
  type Compute = (
    impl Query<Key = u32, Value = ValueChange<u32>>,
    impl Query<Key = u32, Value = u32> + MultiQuery<Key = u32, Value = u32>,
  );
  fn describe(&self, cx: &mut Context) -> Self::Compute {
    let (d, v) = self.inner.describe(cx).resolve_kept();

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
    let f_v = KeyDualMappedQuery {
      base: v.clone(),
      f1: |k: RawEntityHandle| k.index(),
      f2: move |k| RawEntityHandle(allocator.get_handle(k as usize)?).into(),
    };

    let f_v = QueryExt::map(f_v, |_: &u32, v: RawEntityHandle| v.index());

    let allocator = self.allocator.make_read_holder();
    let inv = KeyDualMappedQuery {
      base: v,
      f1: |k: RawEntityHandle| k.index(),
      f2: move |k| RawEntityHandle(allocator.get_handle(k as usize)?).into(),
    };
    let inv = MultiQueryExt::multi_map(inv, |v: RawEntityHandle| v.index());

    let v = OneManyRelationDualAccess {
      many_access_one: f_v,
      one_access_many: inv,
    };

    (d, v)
  }

  fn request(&mut self, request: &mut ReactiveQueryRequest) {
    self.inner.request(request)
  }
}

#[test]
fn test_rev_ref() {
  setup_global_database(Default::default());
  setup_active_reactive_query_registry(Default::default());
  let watch = DatabaseMutationWatch::new(&global_database());
  let rev_watch = DatabaseEntityReverseReference::new(watch.clone());
  register_global_database_feature(watch);
  register_global_database_feature(rev_watch);

  declare_entity!(MyTestEntity);

  global_database().declare_entity::<MyTestEntity>();

  declare_entity!(MyTestEntity2);
  declare_foreign_key!(TestEntity2ReferenceEntity1, MyTestEntity2, MyTestEntity);

  global_database()
    .declare_entity::<MyTestEntity2>()
    .declare_foreign_key::<TestEntity2ReferenceEntity1>();

  let ptr = global_entity_of::<MyTestEntity>()
    .entity_writer()
    .new_entity();

  let _ptr2 = global_entity_of::<MyTestEntity2>()
    .entity_writer()
    .with_component_value_writer::<TestEntity2ReferenceEntity1>(Some(ptr.into()))
    .new_entity();

  let _ptr3 = global_entity_of::<MyTestEntity2>()
    .entity_writer()
    .with_component_value_writer::<TestEntity2ReferenceEntity1>(Some(ptr.into()))
    .new_entity();

  let rev_ref_watch = global_rev_ref().watch_inv_ref::<TestEntity2ReferenceEntity1>();

  noop_ctx!(cx);

  {
    let qcx = QueryResolveCtx::default();
    let (change, dual_access) = rev_ref_watch
      .describe_with_inv_dyn(cx)
      .resolve_one_many_dyn(&qcx);

    assert_eq!(change.iter_key_value().count(), 2);

    let rev_count = dual_access
      .one_access_many
      .access_multi(&ptr)
      .unwrap()
      .count();

    assert_eq!(rev_count, 2);
  }
}
