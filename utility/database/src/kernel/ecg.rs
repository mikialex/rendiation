use crate::*;

pub struct EntityHandle<T> {
  pub(crate) ty: PhantomData<T>,
  pub(crate) handle: RawEntityHandle,
}

impl<T> LinearIdentified for EntityHandle<T> {
  fn alloc_index(&self) -> u32 {
    self.handle.alloc_index()
  }
}

impl<T> EntityHandle<T> {
  pub unsafe fn from_raw(handle: RawEntityHandle) -> Self {
    Self {
      ty: PhantomData,
      handle,
    }
  }
  pub(crate) fn from_raw_internal_usage(handle: RawEntityHandle) -> Self {
    Self {
      ty: PhantomData,
      handle,
    }
  }
  pub fn into_raw(self) -> RawEntityHandle {
    self.handle
  }
  pub fn some_handle(&self) -> Option<RawEntityHandle> {
    Some(self.handle)
  }
}

impl<T> Copy for EntityHandle<T> {}

impl<T> Clone for EntityHandle<T> {
  fn clone(&self) -> Self {
    *self
  }
}
impl<T> PartialEq for EntityHandle<T> {
  fn eq(&self, other: &Self) -> bool {
    self.handle == other.handle
  }
}
impl<T> Eq for EntityHandle<T> {}
impl<T> Hash for EntityHandle<T> {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    self.ty.hash(state);
    self.handle.hash(state);
  }
}
impl<T> std::fmt::Debug for EntityHandle<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("EntityHandle")
      .field("ty", &self.ty)
      .field("handle", &self.handle)
      .finish()
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RawEntityHandle(pub(crate) Handle<()>);

impl LinearIdentified for RawEntityHandle {
  fn alloc_index(&self) -> u32 {
    self.0.index() as u32
  }
}

impl RawEntityHandle {
  pub fn index(&self) -> u32 {
    self.0.index() as u32
  }
}

impl<T> From<EntityHandle<T>> for RawEntityHandle {
  fn from(val: EntityHandle<T>) -> Self {
    val.handle
  }
}

pub struct EntityComponentGroupTyped<E: EntitySemantic> {
  _phantom: SendSyncPhantomData<E>,
  pub(crate) inner: EntityComponentGroup,
}

impl<E: EntitySemantic> Clone for EntityComponentGroupTyped<E> {
  fn clone(&self) -> Self {
    Self {
      _phantom: Default::default(),
      inner: self.inner.clone(),
    }
  }
}

#[derive(Clone)]
pub struct EntityComponentGroup {
  pub(crate) inner: Arc<EntityComponentGroupImpl>,
}

impl EntityComponentGroup {
  pub fn into_typed<E: EntitySemantic>(self) -> Option<EntityComponentGroupTyped<E>> {
    if self.inner.type_id != E::entity_id() {
      return None;
    }
    EntityComponentGroupTyped {
      _phantom: Default::default(),
      inner: self,
    }
    .into()
  }
}

pub(crate) struct EntityComponentGroupImpl {
  pub(crate) type_id: EntityId,
  pub(crate) allocator: Arc<RwLock<Arena<()>>>,
  /// the components of entity
  /// todo consider using small vec or small map?
  pub(crate) components: RwLock<FastHashMap<ComponentId, ComponentCollectionUntyped>>,
  /// the foreign keys of entity, each foreign key express the one-to-many relation with other ECG.
  /// each foreign key is a dependency between different ECG
  ///
  /// components id => foreign id
  pub(crate) foreign_keys: RwLock<FastHashMap<ComponentId, EntityId>>,

  pub(crate) components_meta_watchers: EventSource<ComponentCollectionUntyped>,
  pub(crate) foreign_key_meta_watchers: EventSource<(ComponentId, EntityId)>,

  pub(crate) entity_watchers: EventSource<EntityRangeChange>,
}

impl EntityComponentGroupImpl {
  pub fn new(type_id: EntityId) -> Self {
    Self {
      type_id,
      allocator: Default::default(),
      components: Default::default(),
      foreign_keys: Default::default(),
      components_meta_watchers: Default::default(),
      foreign_key_meta_watchers: Default::default(),
      entity_watchers: Default::default(),
    }
  }
}

impl<E: EntitySemantic> EntityComponentGroupTyped<E> {
  pub fn new(type_id: EntityId) -> Self {
    Self {
      _phantom: Default::default(),
      inner: EntityComponentGroup::new(type_id),
    }
  }
  pub fn into_untyped(self) -> EntityComponentGroup {
    self.inner
  }

  pub fn declare_component<S: ComponentSemantic<Entity = E>>(self) -> Self {
    let com = ComponentCollection::<S>::default();
    let com = ComponentCollectionUntyped {
      inner: Box::new(com),
      data_typeid: TypeId::of::<S::Data>(),
      entity_type_id: S::Entity::entity_id(),
      component_type_id: S::component_id(),
    };
    self.inner.declare_component_dyn(S::component_id(), com);
    self
  }
  pub fn declare_foreign_key<S: ForeignKeySemantic<Entity = E>>(mut self) -> Self {
    self = self.declare_component::<S>();
    self
      .inner
      .declare_foreign_key_dyn(S::component_id(), S::ForeignEntity::entity_id());
    self
  }
  pub fn access_component<S: ComponentSemantic, R>(
    &self,
    f: impl FnOnce(&ComponentCollection<S>) -> R,
  ) -> R {
    self.inner.access_component(S::component_id(), |c| {
      f(c.inner.as_any().downcast_ref().unwrap())
    })
  }
}

impl EntityComponentGroup {
  pub fn new(type_id: EntityId) -> Self {
    Self {
      inner: Arc::new(EntityComponentGroupImpl::new(type_id)),
    }
  }

  pub fn declare_component_dyn(&self, semantic: ComponentId, com: ComponentCollectionUntyped) {
    let mut components = self.inner.components.write();
    self.inner.components_meta_watchers.emit(&com);
    let previous = components.insert(semantic, com);
    assert!(previous.is_none());
  }

  pub fn declare_foreign_key_dyn(&self, semantic: ComponentId, foreign_entity_type_id: EntityId) {
    let mut foreign_keys = self.inner.foreign_keys.write();
    self
      .inner
      .foreign_key_meta_watchers
      .emit(&(semantic, foreign_entity_type_id));
    let previous = foreign_keys.insert(semantic, foreign_entity_type_id);
    assert!(previous.is_none())
  }

  pub fn iter_entity_idx(&self) -> impl Iterator<Item = RawEntityHandle> {
    let inner = self.inner.allocator.make_read_holder();
    struct Iter {
      iter: arena::Iter<'static, ()>,
      _holder: LockReadGuardHolder<Arena<()>>,
    }

    impl Iterator for Iter {
      type Item = RawEntityHandle;

      fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(idx, _)| RawEntityHandle(idx))
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
    f: impl FnOnce(&ComponentCollectionUntyped) -> R,
  ) -> R {
    let components = self.inner.components.read_recursive();
    f(components.get(&c_id).unwrap())
  }
}
