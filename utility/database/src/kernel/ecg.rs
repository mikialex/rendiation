use crate::*;

pub struct EntityHandle<T> {
  pub(crate) ty: PhantomData<T>,
  pub(crate) handle: Handle<()>,
}

impl<T> Copy for EntityHandle<T> {}

impl<T> Clone for EntityHandle<T> {
  fn clone(&self) -> Self {
    *self
  }
}

impl<T> EntityHandle<T> {
  pub fn alloc_idx(&self) -> AllocIdx<T> {
    (self.handle.index() as u32).into()
  }
}

pub struct EntityComponentGroupTyped<E> {
  _phantom: SendSyncPhantomData<E>,
  pub(crate) inner: EntityComponentGroup,
}

impl<E> Clone for EntityComponentGroupTyped<E> {
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
  pub fn into_typed<E: 'static>(self) -> Option<EntityComponentGroupTyped<E>> {
    if self.inner.type_id != TypeId::of::<E>() {
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
  pub(crate) type_id: TypeId,
  pub(crate) allocator: Arc<RwLock<Arena<()>>>,
  /// the components of entity
  pub(crate) components: RwLock<FastHashMap<TypeId, ComponentCollectionUntyped>>,
  /// the foreign keys of entity, each foreign key express the one-to-many relation with other ECG.
  /// each foreign key is a dependency between different ECG
  pub(crate) foreign_keys: RwLock<FastHashMap<TypeId, (TypeId, ComponentCollectionUntyped)>>,

  pub(crate) components_meta_watchers: EventSource<ComponentCollectionUntyped>,
  pub(crate) foreign_key_meta_watchers: EventSource<ComponentCollectionUntyped>,
}

impl EntityComponentGroupImpl {
  pub fn new(type_id: TypeId) -> Self {
    Self {
      type_id,
      allocator: Default::default(),
      components: Default::default(),
      foreign_keys: Default::default(),
      components_meta_watchers: Default::default(),
      foreign_key_meta_watchers: Default::default(),
    }
  }
}

impl<E: 'static> EntityComponentGroupTyped<E> {
  pub fn new(type_id: TypeId) -> Self {
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
      entity_type_id: TypeId::of::<S::Entity>(),
      component_type_id: TypeId::of::<S>(),
    };
    self.inner.declare_component_dyn(TypeId::of::<S>(), com);
    self
  }
  pub fn declare_foreign_key<S: ForeignKeySemantic>(self) -> Self {
    let com = ComponentCollection::<S>::default();
    let com = ComponentCollectionUntyped {
      inner: Box::new(com),
      data_typeid: TypeId::of::<S::Data>(),
      entity_type_id: TypeId::of::<S::Entity>(),
      component_type_id: TypeId::of::<S>(),
    };
    self
      .inner
      .declare_foreign_key_dyn(TypeId::of::<S>(), TypeId::of::<S::ForeignEntity>(), com);
    self
  }
  pub fn access_component<S: ComponentSemantic, R>(
    &self,
    f: impl FnOnce(&ComponentCollection<S>) -> R,
  ) -> R {
    self.inner.access_component(TypeId::of::<S>(), f)
  }
}

impl EntityComponentGroup {
  pub fn new(type_id: TypeId) -> Self {
    Self {
      inner: Arc::new(EntityComponentGroupImpl::new(type_id)),
    }
  }

  pub fn declare_component_dyn(&self, semantic: TypeId, com: ComponentCollectionUntyped) {
    let mut components = self.inner.components.write();
    self.inner.components_meta_watchers.emit(&com);
    let previous = components.insert(semantic, com);
    assert!(previous.is_none());
  }

  pub fn declare_foreign_key_dyn(
    &self,
    semantic: TypeId,
    foreign_entity_type_id: TypeId,
    com: ComponentCollectionUntyped,
  ) {
    let mut foreign_keys = self.inner.foreign_keys.write();
    self.inner.foreign_key_meta_watchers.emit(&com);
    let previous = foreign_keys.insert(foreign_entity_type_id, (semantic, com));
    assert!(previous.is_none())
  }

  pub fn iter_entity_idx(&self) -> impl Iterator<Item = u32> {
    let inner = self.inner.allocator.make_read_holder();
    struct Iter {
      iter: arena::Iter<'static, ()>,
      _holder: LockReadGuardHolder<Arena<()>>,
    }

    impl Iterator for Iter {
      type Item = u32;

      fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|(idx, _)| idx.index() as u32)
      }
    }

    Iter {
      iter: unsafe { std::mem::transmute(inner.iter()) },
      _holder: inner,
    }
  }

  pub fn access_component<R, D: ComponentSemantic>(
    &self,
    c_id: TypeId,
    f: impl FnOnce(&ComponentCollection<D>) -> R,
  ) -> R {
    let components = self.inner.components.read_recursive();
    f(components
      .get(&c_id)
      .unwrap()
      .inner
      .as_any()
      .downcast_ref()
      .unwrap())
  }
}
