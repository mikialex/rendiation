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

pub struct EntityComponentGroup<E> {
  pub(crate) inner: Arc<EntityComponentGroupImpl<E>>,
}

impl<E> Clone for EntityComponentGroup<E> {
  fn clone(&self) -> Self {
    Self {
      inner: self.inner.clone(),
    }
  }
}

pub struct EntityComponentGroupImpl<E> {
  phantom: PhantomData<E>,
  pub(crate) allocator: Arc<RwLock<Arena<()>>>,
  /// the components of entity
  pub(crate) components: RwLock<FastHashMap<TypeId, Box<dyn DynamicComponent>>>,
  /// the foreign keys of entity, each foreign key express the one-to-many relation with other ECG.
  /// each foreign key is a dependency between different ECG
  pub(crate) foreign_keys: RwLock<FastHashMap<TypeId, Box<dyn DynamicComponent>>>,

  pub(crate) components_meta_watchers: EventSource<Box<dyn DynamicComponent>>,
  pub(crate) foreign_key_meta_watchers: EventSource<Box<dyn DynamicComponent>>,
}

impl<E: Any> Default for EntityComponentGroupImpl<E> {
  fn default() -> Self {
    Self {
      phantom: Default::default(),
      allocator: Default::default(),
      components: Default::default(),
      foreign_keys: Default::default(),
      components_meta_watchers: Default::default(),
      foreign_key_meta_watchers: Default::default(),
    }
  }
}

impl<E: Any> Default for EntityComponentGroup<E> {
  fn default() -> Self {
    Self {
      inner: Default::default(),
    }
  }
}

unsafe impl<E> Send for EntityComponentGroupImpl<E> {}
unsafe impl<E> Sync for EntityComponentGroupImpl<E> {}

impl<E: 'static> EntityComponentGroup<E> {
  pub fn declare_component<S: ComponentSemantic<Entity = E>>(self) -> Self {
    let com = ComponentCollection::<S::Data>::default();
    self.declare_component_dyn(TypeId::of::<S>(), Box::new(com));
    self
  }
  pub fn declare_component_dyn(&self, semantic: TypeId, com: Box<dyn DynamicComponent>) {
    let mut components = self.inner.components.write();
    self.inner.components_meta_watchers.emit(&com);
    let previous = components.insert(semantic, com);
    assert!(previous.is_none());
  }
  pub fn declare_foreign_key<FE: Any>(self) -> Self {
    let com = ComponentCollection::<Option<AllocIdx<FE>>>::default();
    self.declare_foreign_key_dyn(TypeId::of::<FE>(), Box::new(com.clone()));
    self
  }
  pub fn declare_foreign_key_dyn(&self, entity_type_id: TypeId, com: Box<dyn DynamicComponent>) {
    let mut foreign_keys = self.inner.foreign_keys.write();
    self.inner.foreign_key_meta_watchers.emit(&com);
    let previous = foreign_keys.insert(entity_type_id, com);
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

  pub fn entity_writer(&self) -> EntityWriter<E> {
    let components = self.inner.components.read_recursive();
    let components = components
      .iter()
      .map(|(id, c)| (*id, c.create_dyn_writer_default()))
      .collect();
    let foreign_keys = self.inner.foreign_keys.read_recursive();
    let foreign_keys = foreign_keys
      .iter()
      .map(|(id, c)| (*id, c.create_dyn_writer_default()))
      .collect();
    EntityWriter {
      phantom: PhantomData,
      components,
      foreign_keys,
      allocator: self.inner.allocator.make_write_holder(),
    }
  }

  pub fn access_component<S: ComponentSemantic, R>(
    &self,
    f: impl FnOnce(&ComponentCollection<S::Data>) -> R,
  ) -> R {
    let components = self.inner.components.read_recursive();
    f(components
      .get(&TypeId::of::<S>())
      .unwrap()
      .as_any()
      .downcast_ref()
      .unwrap())
  }
}

/// Holder the all components write lock, optimized for batch entity creation and modification
pub struct EntityWriter<E> {
  phantom: PhantomData<E>, //
  // todo smallvec
  allocator: LockWriteGuardHolder<Arena<()>>,
  components: Vec<(TypeId, Box<dyn EntityComponentWriter>)>,
  foreign_keys: Vec<(TypeId, Box<dyn EntityComponentWriter>)>,
}

impl<E> EntityWriter<E> {
  pub fn with_component_writer<C: ComponentSemantic, W: EntityComponentWriter + 'static>(
    mut self,
    writer_maker: impl FnOnce(ComponentWriteView<C::Data>) -> W,
  ) -> Self {
    for (id, view) in &mut self.components {
      if *id == TypeId::of::<C>() {
        let v = view.take_write_view();
        let v = v.downcast::<ComponentWriteView<C::Data>>().unwrap();
        *view = Box::new(writer_maker(*v));
        return self;
      }
    }
    self
  }

  pub fn with_foreign_key_writer<FE: Any, W: EntityComponentWriter + 'static>(
    mut self,
    writer_maker: impl FnOnce(ComponentWriteView<Option<AllocIdx<FE>>>) -> W,
  ) -> Self {
    for (id, view) in &mut self.foreign_keys {
      if *id == TypeId::of::<FE>() {
        let v = view.take_write_view();
        let v = v
          .downcast::<ComponentWriteView<Option<AllocIdx<FE>>>>()
          .unwrap();
        *view = Box::new(writer_maker(*v));
        return self;
      }
    }
    self
  }

  pub fn new_entity(&mut self) -> EntityHandle<E> {
    let handle = self.allocator.insert(());
    for com in &mut self.components {
      com.1.write_init_component_value(handle.index() as u32)
    }
    for fk in &mut self.foreign_keys {
      fk.1.write_init_component_value(handle.index() as u32)
    }
    EntityHandle {
      handle,
      ty: PhantomData,
    }
  }

  /// note, the referential integrity is not guaranteed and should be guaranteed by the upper level
  /// implementations
  pub fn delete_entity(&mut self, handle: EntityHandle<E>) {
    let handle = handle.handle;
    self.allocator.remove(handle).unwrap();
    for com in &mut self.components {
      com.1.delete_component(handle.index() as u32)
    }
    for fk in &mut self.foreign_keys {
      fk.1.delete_component(handle.index() as u32)
    }
  }
}

pub trait EntityComponentWriter {
  fn write_init_component_value(&mut self, idx: u32);
  fn delete_component(&mut self, idx: u32);
  fn take_write_view(&mut self) -> Box<dyn Any>;
}

pub struct EntityComponentWriterImpl<T: 'static, F> {
  pub(crate) component: Option<ComponentWriteView<T>>,
  pub(crate) default_value: F,
}

impl<T: CValue + Default, F: FnMut() -> T> EntityComponentWriter
  for EntityComponentWriterImpl<T, F>
{
  fn write_init_component_value(&mut self, idx: u32) {
    let com = self.component.as_mut().unwrap();

    unsafe {
      com.data.grow_at_least(idx as usize);
    }

    com.write_impl(idx.into(), (self.default_value)(), true);
  }
  fn delete_component(&mut self, idx: u32) {
    self.component.as_mut().unwrap().delete(idx.into())
  }
  fn take_write_view(&mut self) -> Box<dyn Any> {
    Box::new(self.component.take().unwrap())
  }
}
