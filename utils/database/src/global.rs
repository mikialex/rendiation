use crate::*;

pub static GLOBAL_DATABASE: parking_lot::RwLock<Option<Database>> = parking_lot::RwLock::new(None);

/// return the previous global database
pub fn setup_global_database(sg: Database) -> Option<Database> {
  GLOBAL_DATABASE.write().replace(sg)
}

pub fn global_database() -> Database {
  GLOBAL_DATABASE.read().as_ref().unwrap().clone()
}

pub fn global_entity_of<E>() -> EntityComponentGroup {
  // global_database().
  todo!()
}
