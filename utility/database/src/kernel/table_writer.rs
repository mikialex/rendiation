use crate::*;

impl ArcTable {
  pub fn entity_writer_dyn(&self) -> TableWriterUntyped {
    let components = self.internal.components.read_recursive();
    // see [Table::allocator] for the lock order
    let allocator = self.internal.allocator.make_write_holder();
    let components = components
      .iter()
      .map(|(id, c)| {
        (
          *id,
          TableWriterImpl {
            component: c.write_untyped(),
            has_write_for_new_entity: false,
          },
        )
      })
      .collect();

    // the start must be emitted after all locks are acquired, so that the scope between start
    // and end is exclusive, and no listener can be added in the middle of the scope.
    let change = ScopedMessage::Start;
    self.internal.entity_watchers.emit(&change);

    TableWriterUntyped {
      type_id: self.internal.type_id,
      components,
      entity_watchers: self.internal.entity_watchers.clone(),
      allocator,
    }
  }
}

pub struct TableWriterUntyped {
  pub(crate) type_id: EntityId,
  pub(crate) allocator: LockWriteGuardHolder<TableAllocator>,
  /// this change ptr type is ScopedValueChange<()>, the lifetime of the ptr is only valid
  /// in the callback scope.
  entity_watchers: EventSource<EntityChangeMessage>,
  components: smallvec::SmallVec<[(ComponentId, TableWriterImpl); 6]>,
}

pub struct EntityInitWriteView<'a> {
  components: &'a mut [(ComponentId, TableWriterImpl)],
  idx: RawEntityHandle,
}

impl<'a> EntityInitWriteView<'a> {
  pub fn write<C: ComponentSemantic>(self, v: &C::Data) -> Self {
    let id = C::component_id();
    let com = self
      .components
      .iter_mut()
      .find(|(i, _)| *i == id)
      .map(|(_, v)| v)
      .expect("unknown component");

    unsafe {
      com.write_init_component_value(self.idx, Some(v as *const C::Data as DataPtr));
      com.has_write_for_new_entity = true;
    }

    self
  }
}

impl Drop for TableWriterUntyped {
  fn drop(&mut self) {
    let change = ScopedMessage::End;
    self.components.clear(); // trigger the components writer's dropper first
    self.entity_watchers.emit(&change);
  }
}

impl TableWriterUntyped {
  pub fn get_component_by_id_mut(&mut self, id: ComponentId) -> Option<&mut TableWriterImpl> {
    self
      .components
      .iter_mut()
      .find(|(i, _)| *i == id)
      .map(|(_, v)| v)
  }
  pub fn get_component_by_id(&self, id: ComponentId) -> Option<&TableWriterImpl> {
    self
      .components
      .iter()
      .find(|(i, _)| *i == id)
      .map(|(_, v)| v)
  }

  pub fn notify_reserve_changes(&mut self, count: usize) {
    self
      .entity_watchers
      .emit(&ScopedMessage::ReserveSpace(count));

    for (_, com) in &mut self.components {
      com.component.notify_reserve_changes(count);
    }
  }

  pub fn new_entity(
    &mut self,
    writer: impl FnOnce(EntityInitWriteView) -> EntityInitWriteView,
  ) -> RawEntityHandle {
    let cap_before = self.allocator.capacity();
    let handle = self.allocator.insert(());
    let cap_after = self.allocator.capacity();

    let handle = RawEntityHandle(handle);

    let change = EntityChange::NewEntityStartCreate(handle);
    let change = ScopedMessage::Message(change);
    self.entity_watchers.emit(&change);

    if cap_after > cap_before {
      for com in &mut self.components {
        unsafe { com.1.resize(cap_after as u32) };
      }
    }

    writer(EntityInitWriteView {
      components: &mut self.components,
      idx: handle,
    });

    for com in &mut self.components {
      if !com.1.has_write_for_new_entity {
        unsafe {
          // safety, the handle has just created.
          com.1.write_init_component_value(handle, None);
        }
      } else {
        com.1.has_write_for_new_entity = false
      }
    }

    let change = EntityChange::NewEntityCreated(handle);
    let change = ScopedMessage::Message(change);
    self.entity_watchers.emit(&change);
    handle
  }

  pub fn clone_entity(&mut self, src: RawEntityHandle) -> RawEntityHandle {
    assert!(self.allocator.contains(src.0));

    let cap_before = self.allocator.capacity();
    let handle = self.allocator.insert(());
    let cap_after = self.allocator.capacity();

    let handle = RawEntityHandle(handle);

    let change = EntityChange::NewEntityStartCreate(handle);
    let change = ScopedMessage::Message(change);
    self.entity_watchers.emit(&change);

    for com in &mut self.components {
      unsafe {
        if cap_after > cap_before {
          com.1.resize(cap_after as u32);
        }
        // safety, the handle is just created.
        com.1.clone_component_value(src, handle);
      }
    }

    let change = EntityChange::NewEntityCreated(handle);
    let change = ScopedMessage::Message(change);
    self.entity_watchers.emit(&change);

    handle
  }

  /// note, the referential integrity is not guaranteed and should be guaranteed by the upper level
  /// implementations
  pub fn delete_entity(&mut self, handle: RawEntityHandle) {
    let cap_before = self.allocator.capacity();
    self.allocator.remove(handle.0).unwrap();
    let cap_after = self.allocator.capacity();
    for com in &mut self.components {
      unsafe {
        // safety, the handle is just got removed, so it's valid for components.
        com.1.delete_component(handle);
        if cap_after < cap_before {
          com.1.resize(cap_after as u32);
        }
      }
    }

    let change = EntityChange::DeleteEntity(handle);
    let change = ScopedMessage::Message(change);
    self.entity_watchers.emit(&change);
  }
}

pub struct TableWriterImpl {
  pub(crate) component: ComponentWriteViewUntyped,
  pub(crate) has_write_for_new_entity: bool,
}

impl TableWriterImpl {
  pub fn get(&self, idx: RawEntityHandle, allocator: &TableAllocator) -> Option<DataPtr> {
    self.component.get(idx, allocator)
  }

  /// # Safety
  ///
  /// See [ComponentStorageReadWriteView::set_value]
  pub unsafe fn write_component(&mut self, idx: RawEntityHandle, src: DataPtr) {
    unsafe {
      self.component.write(idx, src);
    }
  }

  /// # Safety
  ///
  /// See [ComponentStorageReadWriteView::set_value_init]
  pub unsafe fn write_init_component_value(&mut self, idx: RawEntityHandle, data: Option<DataPtr>) {
    unsafe {
      self.component.init(idx, data);
    }
  }

  /// # Safety
  ///
  /// src must point to living data, dst must be a newly allocated slot that not initialized,
  /// see [ComponentStorageReadWriteView::set_value_init]
  pub unsafe fn clone_component_value(&mut self, src: RawEntityHandle, dst: RawEntityHandle) {
    unsafe {
      let src = self.component.get_unchecked(src);
      self.write_init_component_value(dst, Some(src));
    }
  }

  unsafe fn delete_component(&mut self, idx: RawEntityHandle) {
    unsafe {
      self.component.delete(idx);
    }
  }

  /// # Safety
  ///
  /// see [ComponentStorageReadViewBase::resize]
  unsafe fn resize(&mut self, max_cap: u32) {
    unsafe {
      self.component.data.deref_mut().resize(max_cap);
    }
  }
}

#[test]
fn clone_entity_should_init_the_new_slot() {
  declare_entity!(CloneTestEntity);
  declare_component!(CloneTestLinear, CloneTestEntity, u32);
  declare_component!(CloneTestSparse, CloneTestEntity, u32);

  for enable_validation in [true, false] {
    let db = Database::new(enable_validation);
    db.declare_entity::<CloneTestEntity>()
      .declare_component::<CloneTestLinear>()
      .declare_sparse_component::<CloneTestSparse>();

    let (stale, src) = {
      let mut writer = db.entity_writer::<CloneTestEntity>();
      let stale =
        writer.new_entity(|w| w.write::<CloneTestLinear>(&7).write::<CloneTestSparse>(&7));
      let src = writer.new_entity(|w| w.write::<CloneTestLinear>(&7).write::<CloneTestSparse>(&7));
      // leave a deleted slot holding the same value as src, the clone will reuse it
      writer.delete_entity(stale);
      (stale, src)
    };

    let table = db
      .access_table::<CloneTestEntity, _>(|t| t.clone())
      .into_untyped();
    let components = [
      CloneTestLinear::component_id(),
      CloneTestSparse::component_id(),
    ];
    let events = Arc::new(RwLock::new(Vec::new()));
    for c_id in components {
      let events = events.clone();
      table.access_component(c_id, |c| {
        c.data_watchers.on(move |change| {
          if let ScopedMessage::Message(change) = change {
            let record = match &change.change {
              ValueChange::Delta((new, _), old) => {
                (c_id, old.is_none(), unsafe { *(*new as *const u32) })
              }
              ValueChange::Remove(_) => (c_id, false, 0),
            };
            events.write().push(record);
          }
          false
        })
      });
    }

    let cloned = db.entity_writer::<CloneTestEntity>().clone_entity(src);
    assert_eq!(cloned.handle.index(), stale.handle.index());

    let events = events.read();
    assert_eq!(events.len(), components.len());
    for c_id in components {
      assert!(
        events.contains(&(c_id, true, 7)),
        "clone should emit init change, validation: {enable_validation}, events: {events:?}"
      );
    }

    assert_eq!(db.read::<CloneTestLinear>().get(cloned), Some(&7));
    assert_eq!(db.read::<CloneTestSparse>().get(cloned), Some(&7));
  }
}

#[cfg(test)]
fn assert_finish_in_time(name: &str, f: impl FnOnce() + Send + 'static) {
  let (sender, receiver) = std::sync::mpsc::channel();
  std::thread::spawn(move || {
    f();
    let _ = sender.send(());
  });
  let finished = receiver
    .recv_timeout(std::time::Duration::from_secs(5))
    .is_ok();
  assert!(finished, "{name} is dead locked");
}

#[test]
fn concurrent_table_writers_with_entity_set_listener_should_not_deadlock() {
  declare_entity!(WriterTestEntity);
  declare_component!(WriterTestComponent, WriterTestEntity, u32);

  let db = Database::new(true);
  db.declare_entity::<WriterTestEntity>()
    .declare_component::<WriterTestComponent>();

  let table = db
    .access_table::<WriterTestEntity, _>(|t| t.clone())
    .into_untyped();
  let receiver = add_entity_set_listen(
    table.entity_capacity(),
    ArenaAccess(table.internal.allocator.make_read_holder()),
    &table.internal.entity_watchers,
  );

  assert_finish_in_time("concurrent table writers", move || {
    let mut writer = db.entity_writer::<WriterTestEntity>();
    let db_ = db.clone();
    let other_writer = std::thread::spawn(move || {
      db_.entity_writer::<WriterTestEntity>().new_entity(|w| w);
    });
    // make sure the other writer is waiting for the lock
    std::thread::sleep(std::time::Duration::from_millis(100));
    writer.new_entity(|w| w);
    drop(writer);
    other_writer.join().unwrap();
  });

  // the start and end of each writer should be paired, and all changes should be collected
  let mut cx = Context::from_waker(futures::task::noop_waker_ref());
  let Poll::Ready(Some(changes)) = receiver.poll_impl(&mut cx) else {
    panic!("entity set listener should receive the changes");
  };
  assert_eq!(changes.compute_query().len(), 2);
}

#[test]
fn reader_holding_multiple_views_should_not_deadlock_with_table_writer() {
  declare_entity!(ReaderTestEntity);
  declare_component!(ReaderTestComponentA, ReaderTestEntity, u32);
  declare_component!(ReaderTestComponentB, ReaderTestEntity, u32);

  let db = Database::new(true);
  db.declare_entity::<ReaderTestEntity>()
    .declare_component::<ReaderTestComponentA>()
    .declare_component::<ReaderTestComponentB>();

  // the writer acquires the component locks by the map order, so the reader acquires the views
  // by the reverse order to create the lock order conflict
  let table = db
    .access_table::<ReaderTestEntity, _>(|t| t.clone())
    .into_untyped();
  let mut order = Vec::new();
  table.access_components(|components| order.extend(components.keys().copied()));
  let (first, last) = (order[0], *order.last().unwrap());

  assert_finish_in_time("reader holding multiple views", move || {
    let last_view = table.access_component(last, |c| c.read_untyped()).unwrap();
    let db_ = db.clone();
    let writer = std::thread::spawn(move || {
      db_.entity_writer::<ReaderTestEntity>().new_entity(|w| w);
    });
    // make sure the writer is waiting for the lock
    std::thread::sleep(std::time::Duration::from_millis(100));
    let first_view = table.access_component(first, |c| c.read_untyped()).unwrap();
    drop(first_view);
    drop(last_view);
    writer.join().unwrap();
  });
}
