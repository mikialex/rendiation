use crate::*;

static GLOBAL_DATABASE: parking_lot::RwLock<Option<Database>> = parking_lot::RwLock::new(None);

/// return the previous global database
pub fn setup_database(sg: Database) -> Option<Database> {
  GLOBAL_DATABASE.write().replace(sg)
}
