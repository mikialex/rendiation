use crate::*;

/// Holder the all components write lock, optimized for batch entity creation and modification
pub struct EntityWriter<E: EntitySemantic> {
  _phantom: SendSyncPhantomData<E>, //
  inner: EntityWriterUntyped,
}

impl<E: EntitySemantic> EntityComponentGroupTyped<E> {
  pub fn entity_writer(&self) -> EntityWriter<E> {
    self.inner.entity_writer_dyn().into_typed().unwrap()
  }
}

impl EntityComponentGroup {
  pub fn entity_writer_dyn(&self) -> EntityWriterUntyped {
    let components = self.inner.components.read_recursive();
    let components = components
      .iter()
      .map(|(id, c)| (*id, c.inner.create_dyn_writer_default()))
      .collect();

    self.inner.entity_watchers.emit(&ScopedMessage::Start);

    EntityWriterUntyped {
      type_id: self.inner.type_id,
      components,
      entity_watchers: self.inner.entity_watchers.clone(),
      allocator: self.inner.allocator.make_write_holder(),
    }
  }
}

impl<E: EntitySemantic> EntityWriter<E> {
  pub fn into_untyped(self) -> EntityWriterUntyped {
    self.inner
  }
  pub fn with_component_writer<C: ComponentSemantic, W: EntityComponentWriter + 'static>(
    mut self,
    writer_maker: impl FnOnce(ComponentWriteView<C>) -> W,
  ) -> Self {
    for (id, view) in &mut self.inner.components {
      if *id == C::component_id() {
        let v = view.take_write_view();
        let v = v.downcast::<ComponentWriteView<C>>().unwrap();
        *view = Box::new(writer_maker(*v));
        return self;
      }
    }
    self
  }

  pub fn new_entity(&mut self) -> EntityHandle<E> {
    EntityHandle {
      handle: self.inner.new_entity(),
      ty: PhantomData,
    }
  }

  /// note, the referential integrity is not guaranteed and should be guaranteed by the upper level
  /// implementations
  pub fn delete_entity(&mut self, handle: EntityHandle<E>) {
    self.inner.delete_entity(handle.handle.index() as u32)
  }
}

pub struct EntityWriterUntyped {
  type_id: EntityId,
  allocator: LockWriteGuardHolder<Arena<()>>,
  entity_watchers: EventSource<EntityRangeChange>,
  components: smallvec::SmallVec<[(ComponentId, Box<dyn EntityComponentWriter>); 6]>,
}

impl Drop for EntityWriterUntyped {
  fn drop(&mut self) {
    self.entity_watchers.emit(&ScopedMessage::End)
  }
}

impl EntityWriterUntyped {
  pub fn into_typed<E: EntitySemantic>(self) -> Option<EntityWriter<E>> {
    if self.type_id != E::entity_id() {
      return None;
    }
    EntityWriter {
      _phantom: Default::default(),
      inner: self,
    }
    .into()
  }

  pub fn new_entity(&mut self) -> Handle<()> {
    let handle = self.allocator.insert(());
    for com in &mut self.components {
      com.1.write_init_component_value(handle.index() as u32)
    }
    let change = IndexValueChange {
      idx: handle.index() as u32,
      change: ValueChange::Delta((), None),
    };
    self.entity_watchers.emit(&ScopedMessage::Message(change));
    handle
  }

  pub fn clone_entity(&mut self, src: Handle<()>) -> Handle<()> {
    let handle = self.allocator.insert(());
    assert!(self.allocator.contains(src));
    for com in &mut self.components {
      com
        .1
        .clone_component_value(src.index() as u32, handle.index() as u32)
    }
    let change = IndexValueChange {
      idx: handle.index() as u32,
      change: ValueChange::Delta((), None),
    };
    self.entity_watchers.emit(&ScopedMessage::Message(change));
    handle
  }

  /// note, the referential integrity is not guaranteed and should be guaranteed by the upper level
  /// implementations
  pub fn delete_entity(&mut self, handle: u32) {
    let h = self.allocator.get_handle(handle as usize).unwrap();
    self.allocator.remove(h).unwrap();
    for com in &mut self.components {
      com.1.delete_component(handle)
    }
    let change = IndexValueChange {
      idx: handle,
      change: ValueChange::Remove(()),
    };
    self.entity_watchers.emit(&ScopedMessage::Message(change));
  }
}

pub trait EntityComponentWriter {
  fn write_init_component_value(&mut self, idx: u32);
  fn clone_component_value(&mut self, src: u32, dst: u32);
  fn delete_component(&mut self, idx: u32);
  fn take_write_view(&mut self) -> Box<dyn Any>;
}

pub struct EntityComponentWriterImpl<T: ComponentSemantic, F> {
  pub(crate) component: Option<ComponentWriteView<T>>,
  pub(crate) default_value: F,
}

impl<T: ComponentSemantic, F: FnMut() -> T::Data> EntityComponentWriter
  for EntityComponentWriterImpl<T, F>
{
  fn write_init_component_value(&mut self, idx: u32) {
    let com = self.component.as_mut().unwrap();

    unsafe {
      com.data.grow_at_least(idx as usize);
    }

    com.write_impl(idx, (self.default_value)(), true);
  }
  fn clone_component_value(&mut self, src: u32, dst: u32) {
    let com = self.component.as_mut().unwrap();

    unsafe {
      com.data.grow_at_least(dst as usize);
    }

    let src_com = com.data.get(src as usize).unwrap().clone();
    com.write_impl(dst, src_com, true)
  }

  fn delete_component(&mut self, idx: u32) {
    self.component.as_mut().unwrap().delete(idx)
  }
  fn take_write_view(&mut self) -> Box<dyn Any> {
    Box::new(self.component.take().unwrap())
  }
}
