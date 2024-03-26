use crate::*;

pub static GLOBAL_DATABASE: parking_lot::RwLock<Option<Database>> = parking_lot::RwLock::new(None);

/// return the previous global database
pub fn setup_global_database(sg: Database) -> Option<Database> {
  GLOBAL_DATABASE.write().replace(sg)
}

pub fn global_database() -> Database {
  GLOBAL_DATABASE.read().as_ref().unwrap().clone()
}

pub fn global_entity_of<E: EntitySemantic>() -> EntityComponentGroupTyped<E> {
  global_database().access_ecg(|ecg| ecg.clone())
}

pub fn global_entity_component_of<S: ComponentSemantic>() -> ComponentCollection<S> {
  global_entity_of::<S::Entity>()
    .access_component::<S, _>(|c| c.clone())
    .clone()
}
