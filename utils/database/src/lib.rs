use std::{
  any::{Any, TypeId},
  marker::PhantomData,
  sync::Arc,
};

use fast_hash_collection::*;
use parking_lot::RwLock;
use reactive::*;

mod global;
pub use global::*;

#[derive(Default, Clone)]
pub struct Database {
  /// ecg forms a DAG
  pub(crate) ecg_tables: Arc<RwLock<FastHashMap<TypeId, Box<dyn Any + Send + Sync>>>>,
  pub(crate) entity_meta_watcher: EventSource<Box<dyn Any + Send + Sync>>,
}

impl Database {
  pub fn declare_entity<E: Any>(&self) -> EntityComponentGroup<E> {
    let mut tables = self.ecg_tables.write();
    let ecg = EntityComponentGroup::new();
    let boxed: Box<dyn Any + Send + Sync> = Box::new(ecg.clone());
    self.entity_meta_watcher.emit(&boxed);
    let previous = tables.insert(TypeId::of::<E>(), boxed);
    assert!(previous.is_none());
    ecg
  }

  pub fn read<C: ComponentSemantic>(&self) -> ComponentReadView<C::Data> {
    let e_id = TypeId::of::<C::Entity>();
    let tables = self.ecg_tables.read_recursive();
    let ecg = tables.get(&e_id).unwrap();
    let ecg = ecg
      .downcast_ref::<EntityComponentGroup<C::Entity>>()
      .unwrap();
    ecg.get_component::<C>().read()
  }
  pub fn write<C: ComponentSemantic>(&self) -> ComponentWriteView<C::Data> {
    let c_id = TypeId::of::<C::Data>();
    let e_id = TypeId::of::<C::Entity>();
    let tables = self.ecg_tables.read_recursive();
    let ecg = tables.get(&e_id).unwrap();
    todo!()
  }

  pub fn add_entity<E: Any>(&self) -> EntityHandle<E> {
    todo!()
  }
  pub fn remove_entity<E: Any>(&self, handle: EntityHandle<E>) {
    // todo
  }

  pub fn check_integrity(&self) {
    //
  }
}

pub struct EntityHandle<T> {
  handle: PhantomData<T>,
  alloc_index: u32,
}

pub trait ComponentSemantic: Any {
  type Data: CValue;
  type Entity: Any;
}

pub struct EntityComponentGroup<E> {
  inner: Arc<EntityComponentGroupImpl<E>>,
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
  pub(crate) entity_type_id: TypeId,
  //   pub(crate) next_id: u64,
  //   pub(crate) ids: Vec<u64>,
  /// the components of entity
  pub(crate) components: RwLock<FastHashMap<TypeId, Box<dyn Any + Send + Sync>>>,
  /// the foreign keys of entity, each foreign key express the one to many relation with other ECG.
  /// each foreign key is a dependency between different ECG
  pub(crate) foreign_keys: RwLock<FastHashMap<TypeId, Box<dyn Any + Send + Sync>>>,
  pub(crate) ref_counts: RwLock<Vec<usize>>,

  pub(crate) components_meta_watchers: EventSource<Box<dyn Any + Send + Sync>>,
  pub(crate) foreign_key_meta_watchers: EventSource<Box<dyn Any + Send + Sync>>,
}

impl<E: Any> Default for EntityComponentGroupImpl<E> {
  fn default() -> Self {
    Self {
      phantom: Default::default(),
      entity_type_id: TypeId::of::<E>(),
      components: Default::default(),
      foreign_keys: Default::default(),
      ref_counts: Default::default(),
      components_meta_watchers: Default::default(),
      foreign_key_meta_watchers: Default::default(),
    }
  }
}

unsafe impl<E> Send for EntityComponentGroupImpl<E> {}
unsafe impl<E> Sync for EntityComponentGroupImpl<E> {}

impl<E: 'static> EntityComponentGroup<E> {
  pub fn new() -> Self {
    Self {
      inner: Arc::new(EntityComponentGroupImpl::default()),
    }
  }
  pub fn declare_component<S: ComponentSemantic>(self) -> Self {
    let com = ComponentCollection::<S::Data>::default();
    self.declare_component_dyn(TypeId::of::<S>(), Box::new(com));
    self
  }
  pub fn declare_component_dyn(&self, semantic: TypeId, com: Box<dyn Any + Send + Sync>) {
    let mut components = self.inner.components.write();
    self.inner.components_meta_watchers.emit(&com);
    let previous = components.insert(semantic, com);
    assert!(previous.is_none());
  }
  pub fn declare_foreign_key<FE: Any>(self) -> Self {
    let com = ComponentCollection::<AllocIdx<FE>>::default();
    self.declare_foreign_key_dyn(TypeId::of::<FE>(), Box::new(com.clone()));
    self
  }
  pub fn declare_foreign_key_dyn(&self, entity_type_id: TypeId, com: Box<dyn Any + Send + Sync>) {
    let mut foreign_keys = self.inner.foreign_keys.write();
    self.inner.foreign_key_meta_watchers.emit(&com);
    let previous = foreign_keys.insert(entity_type_id, com);
    assert!(previous.is_none())
  }

  pub fn entity_writer(&self) -> EntityWriter<E> {
    todo!()
  }

  pub fn get_component<S: ComponentSemantic>(&self) -> ComponentCollection<S::Data> {
    let c_id = TypeId::of::<S::Data>();
    let components = self.inner.components.read();
    components
      .get(&c_id)
      .unwrap()
      .downcast_ref::<ComponentCollection<S::Data>>()
      .unwrap()
      .clone()
  }
}

/// Holder the all components write lock, optimized for batch entity creation and modification
pub struct EntityWriter<E> {
  phantom: PhantomData<E>, //
  components: Vec<(TypeId, Box<dyn Any>)>,
  foreign_keys: Vec<(TypeId, Box<dyn Any>)>,
}

impl<E> EntityWriter<E> {
  pub fn new_entity(&mut self) -> EntityHandle<E> {
    todo!()
  }

  /// note, the referential integrity is not guaranteed and should be guaranteed by the upper level
  /// implementations
  pub fn delete_entity(&mut self, handle: EntityHandle<E>) {
    //
  }
}

pub struct IndexValueChange<T> {
  pub idx: AllocIdx<T>,
  pub change: ValueChange<T>,
}

#[derive(Clone)]
pub struct ComponentCollection<T> {
  pub(crate) data: Arc<RwLock<Vec<T>>>,
  /// watch this component all change with idx
  pub(crate) group_watchers: EventSource<IndexValueChange<T>>,
}

impl<T> ComponentCollection<T> {
  pub fn read(&self) -> ComponentReadView<T> {
    ComponentReadView {
      data: self.data.make_read_holder(),
    }
  }
  pub fn write(&self) -> ComponentWriteView<T> {
    ComponentWriteView {
      data: self.data.make_write_holder(),
      events: self.group_watchers.lock.make_mutex_write_holder(),
    }
  }
}

impl<T> Default for ComponentCollection<T> {
  fn default() -> Self {
    Self {
      data: Default::default(),
      group_watchers: Default::default(),
    }
  }
}

pub struct ComponentReadView<T: 'static> {
  data: LockReadGuardHolder<Vec<T>>,
}

impl<T: 'static> ComponentReadView<T> {
  pub fn get(&self, idx: AllocIdx<T>) -> &T {
    self.data.get(idx.index as usize).unwrap()
  }
}

pub struct ComponentWriteView<T: 'static> {
  data: LockWriteGuardHolder<Vec<T>>,
  events: MutexGuardHolder<Source<IndexValueChange<T>>>,
}

impl<T: CValue + Default> ComponentWriteView<T> {
  pub fn mutate(&mut self, idx: AllocIdx<T>, new: T) {
    if self.data.len() <= idx.index as usize {
      self.data.resize(idx.index as usize + 1, T::default());
    }
    let com = self.data.get_mut(idx.index as usize).unwrap();
    let previous = std::mem::replace(com, new.clone());

    if previous == new {
      return;
    }

    let change = ValueChange::Delta(new, Some(previous));
    self.events.emit(&IndexValueChange { idx, change });

    // self.data.get(idx.index as usize).unwrap()

    todo!()
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

  //   global_database()
  //     .declare_entity::<MyTestEntity2>()
  //     .declare_component::<TestEntity2FieldA, bool>()
  //     .declare_foreign_key::<MyTestEntity>();

  //   let ptr = global_entity_of::<MyTestEntity>().new_entity(|c| {
  //     c.write_component::<TestEntityFieldA>(todo!());
  //     c.write_component::<TestEntityFieldB>(todo!());
  //     c.write_component::<TestEntityFieldA>(todo!());
  //     // not covered component has written by it's default
  //   });

  //   let ptr = global_entity_of::<MyTestEntity2>().new_entity(|c| {
  //     c.write_foreign_key::<MyTestEntity>(ptr);
  //   });

  //   let single_com_read = ptr.read().read_component::<TestEntity2FieldA>();
  //   ptr.write().write_component::<TestEntity2FieldA>(false); // single write

  //   // batch read
  //   let read_view = global_entity_of::<MyTestEntity>()
  //     .read()
  //     .read_component::<TestEntity2FieldA>();
  //   read_view.get(ptr.idx()).unwrap();
  //   read_view.get(another_ptr.idx()).unwrap();

  //   // batch write
  //   let write_view = global_entity_of::<MyTestEntity>()
  //     .write()
  //     .write_component::<TestEntity2FieldA>();
  //   *write_view.get_mut(ptr.idx()).unwrap() = false;
}
