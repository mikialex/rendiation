use crate::*;

#[derive(Default, Clone)]
pub struct Database {
  /// ecg forms a DAG
  pub(crate) ecg_tables: Arc<RwLock<FastHashMap<TypeId, EntityComponentGroup>>>,
  pub(crate) entity_meta_watcher: EventSource<EntityComponentGroup>,
}

impl Database {
  pub fn declare_entity<E: Any>(&self) -> EntityComponentGroupTyped<E> {
    self
      .declare_entity_dyn(TypeId::of::<E>())
      .into_typed()
      .unwrap()
  }
  pub fn declare_entity_dyn(&self, e_id: TypeId) -> EntityComponentGroup {
    let mut tables = self.ecg_tables.write();
    let ecg = EntityComponentGroup::new(e_id);
    self.entity_meta_watcher.emit(&ecg);
    let previous = tables.insert(e_id, ecg.clone());
    assert!(previous.is_none());
    ecg
  }

  pub fn access_ecg_dyn<R>(&self, e_id: TypeId, f: impl FnOnce(&EntityComponentGroup) -> R) -> R {
    let tables = self.ecg_tables.read_recursive();
    let ecg = tables.get(&e_id).unwrap();
    f(ecg)
  }
  pub fn access_ecg<E: Any, R>(&self, f: impl FnOnce(&EntityComponentGroupTyped<E>) -> R) -> R {
    self.access_ecg_dyn(TypeId::of::<E>(), |c| f(&c.clone().into_typed().unwrap()))
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
  pub fn entity_writer_untyped<E: Any>(&self) -> EntityWriterUntyped {
    self.access_ecg::<E, _>(|e| e.entity_writer().into_untyped())
  }
  pub fn entity_writer_untyped_dyn(&self, e_id: TypeId) -> EntityWriterUntyped {
    self.access_ecg_dyn(e_id, |e| e.entity_writer_dyn())
  }
}

#[test]
fn demo_how_to_use_database_generally() {
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
  declare_foreign_key!(TestEntity2ReferenceEntity1, MyTestEntity2, MyTestEntity);

  global_database()
    .declare_entity::<MyTestEntity2>()
    .declare_component::<TestEntity2FieldA>()
    .declare_foreign_key::<TestEntity2ReferenceEntity1>();

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
  read_view.get(ptr.alloc_idx().index);
  read_view.get(ptr.alloc_idx().index);

  // batch write
  // let write_view =  global_entity_component_of::<TestEntityFieldA>().write().write(idx, new)
}
