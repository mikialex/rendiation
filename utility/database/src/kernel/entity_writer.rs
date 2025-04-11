use std::mem::ManuallyDrop;

use crate::*;

pub trait EntityCustomWrite<E: EntitySemantic> {
  type Writer;
  fn create_writer() -> Self::Writer;
  fn write(self, writer: &mut Self::Writer) -> EntityHandle<E>;
}

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
      .map(|(id, c)| {
        (
          *id,
          EntityComponentWriterImpl {
            component: c.write(),
            next_value_writer: None,
          },
        )
      })
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

  pub fn with_component_value_writer<C>(mut self, value: C::Data) -> Self
  where
    C: ComponentSemantic<Entity = E>,
  {
    self.component_value_writer::<C>(value);
    self
  }

  pub fn component_value_writer<C>(&mut self, value: C::Data) -> &mut Self
  where
    C: ComponentSemantic<Entity = E>,
  {
    self.component_writer::<C, _>(|v| v.with_write_value(value))
  }

  pub fn with_component_writer<C>(
    mut self,
    writer_maker: impl for<'a> FnMut() -> &'a C::Data,
  ) -> Self
  where
    C: ComponentSemantic<Entity = E>,
  {
    self.component_writer::<C>(writer_maker);
    self
  }

  pub fn component_writer<C>(
    &mut self,
    writer_maker: impl for<'a> FnMut() -> &'a C::Data,
  ) -> &mut Self
  where
    C: ComponentSemantic<Entity = E>,
  {
    for (id, view) in &mut self.inner.components {
      if *id == C::component_id() {
        view.next_value_writer = Some(todo!());
        return self;
      }
    }
    self
  }

  pub fn mutate_component_data<C>(
    &mut self,
    idx: EntityHandle<C::Entity>,
    writer: impl FnOnce(&mut C::Data),
  ) -> &mut Self
  where
    C: ComponentSemantic<Entity = E>,
  {
    if let Some(mut data) = self.try_read::<C>(idx) {
      writer(&mut data);
      self.write::<C>(idx, data);
    }
    self
  }

  pub fn write<C>(&mut self, idx: EntityHandle<C::Entity>, data: C::Data) -> &mut Self
  where
    C: ComponentSemantic<Entity = E>,
  {
    for (id, view) in &mut self.inner.components {
      if *id == C::component_id() {
        unsafe {
          let data = ManuallyDrop::new(data.clone());
          view.write_component(
            idx.handle,
            &data as *const ManuallyDrop<C::Data> as *const (),
          );
        }
        break;
      }
    }
    self
  }

  pub fn write_foreign_key<C>(
    &mut self,
    idx: EntityHandle<C::Entity>,
    data: Option<EntityHandle<C::ForeignEntity>>,
  ) -> &mut Self
  where
    C: ForeignKeySemantic<Entity = E>,
  {
    self.write::<C>(idx, data.map(|d| d.handle))
  }

  pub fn try_read<C>(&self, idx: EntityHandle<C::Entity>) -> Option<C::Data>
  where
    C: ComponentSemantic<Entity = E>,
  {
    let mut target = None;
    for (id, view) in &self.inner.components {
      if *id == C::component_id() {
        unsafe {
          let target = &mut target as *mut Option<C::Data> as *mut ();
          view.read_component(idx.handle, target);
        }
        break;
      }
    }
    target
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
    self.inner.delete_entity(handle.handle)
  }
}

pub struct EntityWriterUntyped {
  type_id: EntityId,
  allocator: LockWriteGuardHolder<Arena<()>>,
  entity_watchers: EventSource<EntityRangeChange>,
  components: smallvec::SmallVec<[(ComponentId, EntityComponentWriterImpl); 6]>,
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

  pub fn new_entity(&mut self) -> RawEntityHandle {
    let handle = self.allocator.insert(());
    let handle = RawEntityHandle(handle);
    for com in &mut self.components {
      com.1.write_init_component_value(handle)
    }
    let change = IndexValueChange {
      idx: handle,
      change: ValueChange::Delta((), None),
    };
    self.entity_watchers.emit(&ScopedMessage::Message(change));
    handle
  }

  pub fn clone_entity(&mut self, src: RawEntityHandle) -> RawEntityHandle {
    let handle = self.allocator.insert(());
    let handle = RawEntityHandle(handle);
    assert!(self.allocator.contains(src.0));
    for com in &mut self.components {
      com.1.clone_component_value(src, handle)

      //     unsafe {
      //       com.data.grow_at_least(dst.index() as usize);
      //       let src = EntityHandle::from_raw(src);
      //       let dst = EntityHandle::from_raw(dst);

      //       let src_com = com.get(src).unwrap().clone();
      //       com.write_impl(dst, src_com, true)
      //     }
    }
    let change = IndexValueChange {
      idx: handle,
      change: ValueChange::Delta((), None),
    };
    self.entity_watchers.emit(&ScopedMessage::Message(change));
    handle
  }

  /// note, the referential integrity is not guaranteed and should be guaranteed by the upper level
  /// implementations
  pub fn delete_entity(&mut self, handle: RawEntityHandle) {
    self.allocator.remove(handle.0).unwrap();
    for com in &mut self.components {
      com.1.delete_component(handle)
    }
    let change = IndexValueChange {
      idx: handle,
      change: ValueChange::Remove(()),
    };
    self.entity_watchers.emit(&ScopedMessage::Message(change));
  }

  /// note, the referential integrity is not guaranteed and should be guaranteed by the upper level
  /// implementations
  pub fn uncheck_handle_delete_entity(&mut self, handle: u32) {
    let handle = self.allocator.get_handle(handle as usize).unwrap();
    self.delete_entity(RawEntityHandle(handle));
  }
}

// pub trait ComponentInitValueProvider {
//   fn next_init(&mut self) -> DataPtr;
// }

pub struct EntityComponentWriterImpl {
  pub(crate) component: Box<dyn ComponentStorageReadWriteView>,
  pub(crate) next_value_writer: Option<Box<dyn FnMut() -> DataPtr>>,
}

// impl<T, F> EntityComponentReader for EntityComponentWriterImpl<T, F>
// where
//   T: ComponentSemantic,
// {
//   unsafe fn read_component(&self, idx: RawEntityHandle, target: *mut ()) {
//     let target = &mut *(target as *mut Option<T::Data>);
//     let idx = EntityHandle::from_raw(idx);
//     if let Some(data) = self.component.as_ref().unwrap().read(idx) {
//       *target = Some(data);
//     }
//   }
//   fn read_component_into_boxed(&self, idx: RawEntityHandle) -> Option<Box<dyn Any>> {
//     let idx = unsafe { EntityHandle::from_raw(idx) };
//     self
//       .component
//       .as_ref()
//       .unwrap()
//       .read(idx)
//       .map(|v| Box::new(v) as Box<dyn Any>)
//   }
// }

// impl<T, F> EntityComponentWriter for EntityComponentWriterImpl<T, F>
// where
//   T: ComponentSemantic,
//   F: FnMut() -> T::Data,
// {
//   unsafe fn write_component(&mut self, idx: RawEntityHandle, src: *const ()) {
//     let src = &*(src as *const T::Data);
//     let src = std::ptr::read(src);
//     let idx = EntityHandle::from_raw(idx);
//     self.component.as_mut().unwrap().write(idx, src);
//   }

//   fn write_init_component_value(&mut self, idx: RawEntityHandle) {
//     let com = self.component.as_mut().unwrap();

//     unsafe {
//       com.data.grow_at_least(idx.index() as usize);
//       let idx = EntityHandle::from_raw(idx);
//       com.write_impl(idx, (self.default_value)(), true);
//     }
//   }
//   fn clone_component_value(&mut self, src: RawEntityHandle, dst: RawEntityHandle) {
//     let com = self.component.as_mut().unwrap();

//     unsafe {
//       com.data.grow_at_least(dst.index() as usize);
//       let src = EntityHandle::from_raw(src);
//       let dst = EntityHandle::from_raw(dst);

//       let src_com = com.get(src).unwrap().clone();
//       com.write_impl(dst, src_com, true)
//     }
//   }

//   fn take_write_view(&mut self) -> Box<dyn Any> {
//     Box::new(self.component.take().unwrap())
//   }
// }
