use std::hash::Hasher;

use crate::*;

struct LabelHashMarker;

pub struct LabelOf<T>(T);
impl<T: EntitySemantic> EntityAssociateSemantic for LabelOf<T> {
  type Entity = T;
}
impl<T: EntitySemantic> ComponentSemantic for LabelOf<T> {
  type Data = String;

  fn component_id() -> ComponentId {
    compute_component_id(TypeId::of::<T>())
  }
}

fn compute_component_id(entity_type_id: TypeId) -> ComponentId {
  let mut hasher = FastHasher::default();
  entity_type_id.hash(&mut hasher);
  LabelHashMarker.type_id().hash(&mut hasher);
  ComponentId::Hash(hasher.finish())
}

impl Database {
  /// add user defined label(maybe used for debug purpose) for every(and in future) entity
  /// in this db.
  pub fn enable_label_for_all_entity(&self) {
    self.entity_meta_watcher.on(|ecg| {
      ecg.inner.add_label_component();
      false
    });

    for ecg in self.ecg_tables.read().values() {
      ecg.inner.add_label_component();
    }
  }
}

impl EntityComponentGroupImpl {
  fn add_label_component(&self) {
    let semantic = compute_component_id(self.type_id.0);

    let data = Arc::new(RwLock::new(DBDefaultLinearStorage::<String> {
      data: Default::default(),
      default_value: Default::default(),
      old_value_out: Default::default(),
    }));

    let display_name = format!("{} Label", &self.name);

    let com = ComponentCollectionUntyped {
      name: Arc::new(display_name),
      as_foreign_key: None,
      data_typeid: TypeId::of::<String>(),
      entity_type_id: self.type_id,
      component_type_id: semantic,
      data: Arc::new(data),
      allocator: self.allocator.clone(),
      data_watchers: Default::default(),
    };

    self.declare_component_dyn(semantic, com);
  }
}
