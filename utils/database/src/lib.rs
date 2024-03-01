use std::{
  any::{Any, TypeId},
  marker::PhantomData,
  sync::Arc,
};

use fast_hash_collection::*;
use parking_lot::RwLock;
use reactive::*;
use storage::LinkListPool;

pub struct Database {
  /// each ecg forms a DAG
  pub(crate) ecg_tables: Arc<RwLock<FastHashMap<TypeId, EntityComponentGroup>>>,
  pub(crate) entity_meta_watcher: EventSource<EntityComponentGroup>,
}

impl Database {
  pub fn declare_entity<E: Any>(&self) -> EntityComponentGroup {
    self.declare_entity_dyn(TypeId::of::<E>())
  }

  pub fn declare_entity_dyn(&self, type_id: TypeId) -> EntityComponentGroup {
    let mut tables = self.ecg_tables.write();
    let ecg = EntityComponentGroup::new(type_id);
    let previous = tables.insert(type_id, ecg.clone());
    assert!(previous.is_none());
    self.entity_meta_watcher.emit(&ecg);
    ecg
  }

  pub fn read<C: ComponentSemantic>(&self) {
    // todo
  }
  pub fn write<C: ComponentSemantic>(&self) {
    // todo
  }

  pub fn add_entity<E: Any>(&self) -> EntityHandle<E> {
    todo!()
  }
  pub fn remove_entity<E: Any>(&self, handle: EntityHandle<E>) {
    // todo
  }
}

pub struct EntityHandle<T> {
  handle: PhantomData<T>,
}

pub trait ComponentSemantic: Any {
  type Data: CValue;
  type Entity: Any;
}

#[derive(Clone)]
pub struct EntityComponentGroup {
  inner: Arc<EntityComponentGroupImpl>,
}

pub struct EntityComponentGroupImpl {
  pub(crate) entity_type_id: TypeId,
  //   pub(crate) next_id: u64,
  //   pub(crate) ids: Vec<u64>,
  /// the components of entity
  pub(crate) components: RwLock<FastHashMap<TypeId, Box<dyn Any>>>,
  /// the foreign keys of entity, each foreign key express the one to many relation with other ECG.
  /// each foreign key is dependency between the different ECG
  pub(crate) foreign_keys: RwLock<FastHashMap<TypeId, Box<dyn Any>>>,
  pub(crate) ref_counts: RwLock<Vec<usize>>,

  pub(crate) components_meta_watchers: EventSource<Box<dyn Any>>,
  pub(crate) foreign_key_meta_watchers: EventSource<Box<dyn Any>>,
}

impl EntityComponentGroupImpl {
  pub fn new(type_id: TypeId) -> Self {
    Self {
      entity_type_id: type_id,
      components: Default::default(),
      foreign_keys: Default::default(),
      ref_counts: Default::default(),
      components_meta_watchers: Default::default(),
      foreign_key_meta_watchers: Default::default(),
    }
  }
}

impl EntityComponentGroup {
  pub fn new(type_id: TypeId) -> Self {
    Self {
      inner: Arc::new(EntityComponentGroupImpl::new(type_id)),
    }
  }
  pub fn declare_component<S: ComponentSemantic>(self) -> Self {
    let com = ComponentCollection::<S::Data>::default();
    self.declare_component_dyn(TypeId::of::<S>(), Box::new(com));
    self
  }
  pub fn declare_component_dyn(&self, semantic: TypeId, com: Box<dyn Any>) {
    let mut components = self.inner.components.write();
    self.inner.components_meta_watchers.emit(&com);
    let previous = components.insert(semantic, com);
    assert!(previous.is_none());
  }
  pub fn declare_foreign_key<E: Any>(self) -> Self {
    let com = ComponentCollection::<AllocIdx<E>>::default();
    self.declare_foreign_key_dyn(TypeId::of::<E>(), Box::new(com.clone()));
    self
  }
  pub fn declare_foreign_key_dyn(&self, entity_type_id: TypeId, com: Box<dyn Any>) {
    let mut foreign_keys = self.inner.foreign_keys.write();
    self.inner.foreign_key_meta_watchers.emit(&com);
    let previous = foreign_keys.insert(entity_type_id, com);
    assert!(previous.is_none())
  }
}

#[derive(Clone)]
pub struct ComponentCollection<T> {
  pub(crate) data: Arc<RwLock<Vec<T>>>,
  /// watch this component change with given idx
  pub(crate) entity_watchers: Arc<RwLock<LinkListPool<EventListener<T>>>>,
  /// watch this component all change with idx
  pub(crate) group_watchers: EventSource<(u32, T)>,
}

impl<T> Default for ComponentCollection<T> {
  fn default() -> Self {
    Self {
      data: Default::default(),
      entity_watchers: Default::default(),
      group_watchers: Default::default(),
    }
  }
}

// fn demo() {
//   init_global_database(Default::default());

//   global_database()
//     .declare_entity::<MyTestEntity>()
//     .declare_component::<TestEntityFieldA, Mat4<f32>>()
//     .declare_component::<TestEntityFieldB, u32>()
//     .declare_component::<TestEntityFieldC, u32>();

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
// }
