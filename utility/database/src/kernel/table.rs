use crate::*;

#[derive(Clone)]
pub struct ArcTable {
  pub(crate) internal: Arc<Table>,
}

impl std::ops::Deref for ArcTable {
  type Target = Table;

  fn deref(&self) -> &Self::Target {
    &self.internal
  }
}

impl ArcTable {
  pub fn new(type_id: EntityId, name: String, name_mapping: Arc<RwLock<DBNameMapping>>) -> Self {
    Self {
      internal: Arc::new(Table::new(type_id, name, name_mapping)),
    }
  }

  pub fn declare_component_dyn(&self, semantic: ComponentId, com: ComponentUntyped) {
    self.internal.declare_component_dyn(semantic, com);
  }

  pub fn declare_foreign_key_dyn(&self, semantic: ComponentId, foreign_entity_type_id: EntityId) {
    let mut foreign_keys = self.internal.foreign_keys.write();
    self
      .internal
      .foreign_key_meta_watchers
      .emit(&(semantic, foreign_entity_type_id));
    let previous = foreign_keys.insert(semantic, foreign_entity_type_id);
    assert!(previous.is_none())
  }

  pub fn iter_entity_idx(&self) -> impl Iterator<Item = RawEntityHandle> {
    let inner = self.internal.allocator.make_read_holder();
    struct Iter {
      iter: arena::Iter<'static, ()>,
      _holder: LockReadGuardHolder<Arena<()>>,
    }

    impl Iterator for Iter {
      type Item = RawEntityHandle;

      fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(idx, _)| RawEntityHandle(idx))
      }

      fn size_hint(&self) -> (usize, Option<usize>) {
        self.iter.size_hint()
      }
    }

    Iter {
      iter: unsafe { std::mem::transmute(inner.iter()) },
      _holder: inner,
    }
  }

  pub fn access_component<R>(
    &self,
    c_id: ComponentId,
    f: impl FnOnce(&ComponentUntyped) -> R,
  ) -> Option<R> {
    let components = self.internal.components.read_recursive();
    Some(f(components.get(&c_id)?))
  }

  pub fn name(&self) -> &str {
    &self.internal.name
  }

  pub fn short_name(&self) -> &str {
    &self.internal.short_name
  }

  pub fn get_handle_at(&self, index: usize) -> Option<RawEntityHandle> {
    let inner = self.internal.allocator.make_read_holder();
    let handle = inner.get_handle(index)?;
    Some(RawEntityHandle(handle))
  }

  pub fn living_entity_count(&self) -> usize {
    self.internal.allocator.read().len()
  }

  pub fn entity_capacity(&self) -> usize {
    self.internal.allocator.read().capacity()
  }

  pub fn memory_usage_in_bytes(&self) -> usize {
    let mut byte_count = 0;
    byte_count += self.internal.allocator.read().memory_usage_in_bytes();

    let components = self.internal.components.read();
    for c in components.values() {
      byte_count += c.data.memory_usage_in_bytes()
    }

    byte_count
  }

  pub fn component_count(&self) -> usize {
    self.internal.components.read().len()
  }

  pub fn access_components(&self, f: impl FnOnce(&FastHashMap<ComponentId, ComponentUntyped>)) {
    f(&self.internal.components.read());
  }

  pub fn into_typed<E: EntitySemantic>(self) -> Option<EntityComponentGroupTyped<E>> {
    if self.internal.type_id != E::entity_id() {
      return None;
    }
    EntityComponentGroupTyped {
      phantom: Default::default(),
      inner: self,
    }
    .into()
  }
}

pub struct Table {
  /// the name of this entity, will be unique among all components
  pub(crate) name: String,
  pub(crate) short_name: String,
  pub(crate) type_id: EntityId,
  pub(crate) allocator: Arc<RwLock<Arena<()>>>,
  /// The components of entity
  pub(crate) components: RwLock<FastHashMap<ComponentId, ComponentUntyped>>,
  /// The foreign keys of entity, each foreign key express the one-to-many (or possibly one-to-one)
  /// relation with other table. Each foreign key is a dependency between different table
  ///
  /// components id => foreign id
  pub(crate) foreign_keys: RwLock<FastHashMap<ComponentId, EntityId>>,

  pub(crate) components_meta_watchers: EventSource<ComponentUntyped>,
  pub(crate) foreign_key_meta_watchers: EventSource<(ComponentId, EntityId)>,

  pub(crate) entity_watchers: EventSource<ChangePtr>,
  pub(crate) name_mapping: Arc<RwLock<DBNameMapping>>,
}

impl Table {
  pub fn new(type_id: EntityId, name: String, name_mapping: Arc<RwLock<DBNameMapping>>) -> Self {
    Self {
      short_name: disqualified::ShortName(&name).to_string(),
      name,
      type_id,
      allocator: Default::default(),
      components: Default::default(),
      foreign_keys: Default::default(),
      components_meta_watchers: Default::default(),
      foreign_key_meta_watchers: Default::default(),
      entity_watchers: Default::default(),
      name_mapping,
    }
  }

  #[inline(never)]
  pub fn declare_component_dyn(&self, semantic: ComponentId, com: ComponentUntyped) {
    let mut components = self.components.write();

    self
      .name_mapping
      .write()
      .insert_component(semantic, self.type_id, com.name.clone());

    self.components_meta_watchers.emit(&com);
    let previous = components.insert(semantic, com);
    assert!(previous.is_none());
  }
}

pub struct EntityComponentGroupTyped<E: EntitySemantic> {
  phantom: PhantomData<E>,
  pub(crate) inner: ArcTable,
}

impl<E: EntitySemantic> Clone for EntityComponentGroupTyped<E> {
  fn clone(&self) -> Self {
    Self {
      phantom: Default::default(),
      inner: self.inner.clone(),
    }
  }
}

impl<E: EntitySemantic> EntityComponentGroupTyped<E> {
  pub fn new(type_id: EntityId, name: String, name_mapping: Arc<RwLock<DBNameMapping>>) -> Self {
    Self {
      phantom: Default::default(),
      inner: ArcTable::new(type_id, name, name_mapping),
    }
  }

  pub fn into_untyped(self) -> ArcTable {
    self.inner
  }

  pub fn declare_component<S: ComponentSemantic<Entity = E>>(self) -> Self {
    self.declare_component_impl::<S>(None, init_linear_storage::<S>())
  }

  pub fn declare_sparse_component<S: ComponentSemantic<Entity = E>>(self) -> Self {
    self.declare_component_impl::<S>(None, init_sparse_storage::<S>())
  }

  pub fn declare_component_impl<S: ComponentSemantic<Entity = E>>(
    self,
    as_foreign_key: Option<EntityId>,
    storage: impl ComponentStorage + 'static,
  ) -> Self {
    let com = ComponentUntyped {
      short_name: disqualified::ShortName(S::unique_name()).to_string(),
      name: S::unique_name().to_string(),
      as_foreign_key,
      data_meta: storage.create_meta(),
      entity_type_id: S::Entity::entity_id(),
      component_type_id: S::component_id(),
      data: Box::new(storage),
      allocator: self.inner.internal.allocator.clone(),
      data_watchers: Default::default(),
    };
    self.inner.declare_component_dyn(S::component_id(), com);
    self
  }

  pub fn declare_foreign_key<S: ForeignKeySemantic<Entity = E>>(mut self) -> Self {
    self = self.declare_component_impl::<S>(
      S::ForeignEntity::entity_id().into(),
      init_linear_storage::<S>(),
    );
    self
      .inner
      .declare_foreign_key_dyn(S::component_id(), S::ForeignEntity::entity_id());
    self
  }

  pub fn declare_sparse_foreign_key_maybe_sparse<S: ForeignKeySemantic<Entity = E>>(
    mut self,
    sparse: bool,
  ) -> Self {
    if sparse {
      let storage = init_sparse_storage::<S>();
      self = self.declare_component_impl::<S>(S::ForeignEntity::entity_id().into(), storage);
    } else {
      let storage = init_linear_storage::<S>();
      self = self.declare_component_impl::<S>(S::ForeignEntity::entity_id().into(), storage);
    };

    self
      .inner
      .declare_foreign_key_dyn(S::component_id(), S::ForeignEntity::entity_id());
    self
  }

  pub fn access_component<S: ComponentSemantic, R>(
    &self,
    f: impl FnOnce(ComponentCollection<S>) -> R,
  ) -> R {
    if let Some(r) = self
      .inner
      .access_component(S::component_id(), |s| f(unsafe { s.as_typed() }))
    {
      r
    } else {
      panic!(
        "access not exist component {}, make sure declared before use",
        std::any::type_name::<S>()
      );
    }
  }
}
