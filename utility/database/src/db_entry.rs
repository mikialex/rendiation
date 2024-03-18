use crate::*;

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

  global_database().interleave_component_storages(|builder| {
    builder
      .with_type::<TestEntityFieldA>()
      .with_type::<TestEntityFieldB>()
      .with_type::<TestEntityFieldC>()
  });

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
