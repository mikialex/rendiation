use crate::*;

#[derive(Default)]
pub struct DataBaseWithFeatures {
  pub database: Database,
  pub features: DataBaseFeatureGroup,
}

pub static GLOBAL_DATABASE: parking_lot::RwLock<Option<DataBaseWithFeatures>> =
  parking_lot::RwLock::new(None);

/// return the previous global database
pub fn setup_global_database(sg: DataBaseWithFeatures) -> Option<DataBaseWithFeatures> {
  GLOBAL_DATABASE.write().replace(sg)
}

pub fn global_database() -> Database {
  GLOBAL_DATABASE.read().as_ref().unwrap().database.clone()
}

pub fn register_global_database_feature(feature: impl DataBaseFeatureBox) {
  GLOBAL_DATABASE
    .write()
    .as_mut()
    .unwrap()
    .features
    .register_feature(feature);
}

pub fn global_entity_of<E: EntitySemantic>() -> EntityComponentGroupTyped<E> {
  global_database().access_ecg(|ecg| ecg.clone())
}

pub fn global_entity_component_of<S: ComponentSemantic>() -> ComponentCollection<S> {
  global_entity_of::<S::Entity>()
    .access_component::<S, _>(|c| c.clone())
    .clone()
}

pub fn global_watch() -> DatabaseMutationWatch {
  GLOBAL_DATABASE
    .read()
    .as_ref()
    .unwrap()
    .features
    .get_feature::<DatabaseMutationWatch>()
    .clone()
}

pub fn global_rev_ref() -> DatabaseEntityReverseReference {
  GLOBAL_DATABASE
    .read()
    .as_ref()
    .unwrap()
    .features
    .get_feature::<DatabaseEntityReverseReference>()
    .clone()
}
