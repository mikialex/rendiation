#![feature(alloc_layout_extra)]

use std::{
  any::{Any, TypeId},
  marker::PhantomData,
  ops::Deref,
  sync::Arc,
};

use arena::*;
use fast_hash_collection::*;
use parking_lot::RwLock;
use reactive::*;

mod global;
pub use global::*;

mod storage;
pub use storage::*;

#[derive(Default, Clone)]
pub struct Database {
  /// ecg forms a DAG
  pub(crate) ecg_tables: Arc<RwLock<FastHashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
  pub(crate) entity_meta_watcher: EventSource<Box<dyn Any + Send + Sync>>,
}

impl Database {
  pub fn declare_entity<E: Any>(&self) -> EntityComponentGroup<E> {
    let mut tables = self.ecg_tables.write();
    let ecg = EntityComponentGroup::default();
    let boxed: Box<dyn Any + Send + Sync> = Box::new(ecg.clone());
    self.entity_meta_watcher.emit(&boxed);
    let previous = tables.insert(TypeId::of::<E>(), boxed);
    assert!(previous.is_none());
    ecg
  }

  fn access_ecg<E: Any, R>(&self, f: impl FnOnce(&EntityComponentGroup<E>) -> R) -> R {
    let e_id = TypeId::of::<E>();
    let tables = self.ecg_tables.read_recursive();
    let ecg = tables.get(&e_id).unwrap();
    let ecg = ecg.downcast_ref::<EntityComponentGroup<E>>().unwrap();
    f(ecg)
  }

  pub fn read<C: ComponentSemantic>(&self) -> ComponentReadView<C::Data> {
    self.access_ecg::<C::Entity, _>(|e| e.access_component::<C, _>(|c| c.read()))
  }
  pub fn write<C: ComponentSemantic>(&self) -> ComponentWriteView<C::Data> {
    self.access_ecg::<C::Entity, _>(|e| e.access_component::<C, _>(|c| c.write()))
  }

  pub fn entity_writer<E: Any>(&self) -> EntityWriter<E> {
    self.access_ecg::<E, _>(|e| e.entity_writer())
  }
}

pub struct EntityHandle<T> {
  ty: PhantomData<T>,
  handle: Handle<()>,
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

pub trait ComponentSemantic: Any {
  type Data: CValue + Default;
  type Entity: Any;
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
  /// the foreign keys of entity, each foreign key express the one to many relation with other ECG.
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

pub trait EntityComponentWriter {
  fn write_init_component_value(&mut self, idx: u32);
  fn delete_component(&mut self, idx: u32);
  fn take_write_view(&mut self) -> Box<dyn Any>;
}

pub struct EntityComponentWriterImpl<T: 'static, F> {
  component: Option<ComponentWriteView<T>>,
  default_value: F,
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

pub trait DynamicComponent: Any + Send + Sync {
  fn create_dyn_writer_default(&self) -> Box<dyn EntityComponentWriter>;
  fn setup_new_storage(&mut self, storage: Box<dyn Any>);
  fn as_any(&self) -> &dyn Any;
}

impl<T: CValue + Default> DynamicComponent for ComponentCollection<T> {
  fn create_dyn_writer_default(&self) -> Box<dyn EntityComponentWriter> {
    Box::new(self.write().with_writer(T::default))
  }
  fn setup_new_storage(&mut self, storage: Box<dyn Any>) {
    self.data = *storage.downcast::<Arc<dyn ComponentStorage<T>>>().unwrap();
  }

  fn as_any(&self) -> &dyn Any {
    self
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

pub struct IndexValueChange<T> {
  pub idx: AllocIdx<T>,
  pub change: ValueChange<T>,
}

#[derive(Clone)]
pub struct ComponentCollection<T> {
  // todo make this optional static dispatch for better performance
  // todo remove arc
  pub(crate) data: Arc<dyn ComponentStorage<T>>,
  /// watch this component all change with idx
  pub(crate) group_watchers: EventSource<IndexValueChange<T>>,
}

impl<T> ComponentCollection<T> {
  pub fn read(&self) -> ComponentReadView<T> {
    ComponentReadView {
      data: self.data.create_read_view(),
    }
  }
  pub fn write(&self) -> ComponentWriteView<T> {
    ComponentWriteView {
      data: self.data.create_read_write_view(),
      events: self.group_watchers.lock.make_mutex_write_holder(),
    }
  }
}

impl<T: CValue + Default> Default for ComponentCollection<T> {
  fn default() -> Self {
    let data: Arc<RwLock<Vec<T>>> = Default::default();
    Self {
      data: Arc::new(data),
      group_watchers: Default::default(),
    }
  }
}

pub struct ComponentReadView<T: 'static> {
  data: Box<dyn ComponentStorageReadView<T>>,
}

impl<T: 'static> ComponentReadView<T> {
  pub fn get(&self, idx: AllocIdx<T>) -> &T {
    self.data.get(idx.index as usize).unwrap()
  }
}

pub struct ComponentWriteView<T: 'static> {
  data: Box<dyn ComponentStorageReadWriteView<T>>,
  events: MutexGuardHolder<Source<IndexValueChange<T>>>,
}

impl<T: CValue + Default> ComponentWriteView<T> {
  pub fn with_writer(self, f: impl FnMut() -> T + 'static) -> impl EntityComponentWriter {
    EntityComponentWriterImpl {
      component: Some(self),
      default_value: f,
    }
  }

  pub fn write(&mut self, idx: AllocIdx<T>, new: T) {
    self.write_impl(idx, new, false);
  }

  fn write_impl(&mut self, idx: AllocIdx<T>, new: T, is_create: bool) {
    let com = self.data.get_mut(idx.index as usize).unwrap();
    let previous = std::mem::replace(com, new.clone());

    if is_create {
      let change = ValueChange::Delta(new, None);
      self.events.emit(&IndexValueChange { idx, change });
      return;
    }

    if previous == new {
      return;
    }

    let change = ValueChange::Delta(new, Some(previous));
    self.events.emit(&IndexValueChange { idx, change });
  }

  fn delete(&mut self, idx: AllocIdx<T>) {
    let com = self.data.get_mut(idx.index as usize).unwrap();
    let previous = std::mem::take(com);

    let change = ValueChange::Remove(previous);
    self.events.emit(&IndexValueChange { idx, change });
  }
}

#[macro_export]
macro_rules! declare_component {
  ($Type: tt, $EntityTy: ty, $DataTy: ty) => {
    pub struct $Type;
    impl ComponentSemantic for $Type {
      type Data = $DataTy;
      type Entity = $EntityTy;
    }
  };
}

#[test]
fn demo() {
  setup_global_database(Default::default());

  pub struct MyTestEntity;
  declare_component!(TestEntityFieldA, MyTestEntity, (f32, f32));
  declare_component!(TestEntityFieldB, MyTestEntity, f32);
  declare_component!(TestEntityFieldC, MyTestEntity, f32);

  global_database()
    .declare_entity::<MyTestEntity>()
    .declare_component::<TestEntityFieldA>()
    .declare_component::<TestEntityFieldB>()
    .declare_component::<TestEntityFieldC>();

  // global_database().interleave_component_storages(|builder| {
  //   builder
  //     .with_type::<TestEntityFieldA>()
  //     .with_type::<TestEntityFieldB>()
  //     .with_type::<TestEntityFieldC>()
  // });

  pub struct MyTestEntity2;
  declare_component!(TestEntity2FieldA, MyTestEntity2, u32);

  global_database()
    .declare_entity::<MyTestEntity2>()
    .declare_component::<TestEntity2FieldA>()
    .declare_foreign_key::<MyTestEntity>();

  let ptr = global_entity_of::<MyTestEntity>()
    .entity_writer()
    .with_component_writer::<TestEntityFieldB, _>(|w| w.with_writer(|| 1.))
    .new_entity();

  let ptr = global_entity_of::<MyTestEntity2>()
    .entity_writer()
    .with_foreign_key_writer::<MyTestEntity, _>(move |w| {
      w.with_writer(move || Some(ptr.alloc_idx()))
    })
    .new_entity();

  //   let single_com_read = ptr.read().read_component::<TestEntity2FieldA>();
  //   ptr.write().write_component::<TestEntity2FieldA>(false); // single write

  // batch read
  let read_view = global_entity_component_of::<TestEntity2FieldA>().read();
  read_view.get(ptr.alloc_idx().index.into());
  read_view.get(ptr.alloc_idx().index.into());

  // batch write
  // let write_view =  global_entity_component_of::<TestEntityFieldA>().write().write(idx, new)
}
