use std::{
  any::{Any, TypeId},
  sync::Arc,
};

use fast_hash_collection::FastHashMap;
use parking_lot::RwLock;

use crate::*;

pub type RxCForker<K, V> = ReactiveKVMapFork<Box<dyn DynamicReactiveCollection<K, V>>, K, V>;
pub type OneManyRelationForker<O, M> = OneToManyRefDenseBookKeeping<O, M, RxCForker<M, O>>;

#[derive(Default, Clone)]
pub struct CollectionRegistry {
  registry: Arc<RwLock<FastHashMap<TypeId, Box<dyn Any>>>>,
  relations: Arc<RwLock<FastHashMap<TypeId, Box<dyn Any>>>>,
}

// todo
unsafe impl Send for CollectionRegistry {}
unsafe impl Sync for CollectionRegistry {}

static ACTIVE_REGISTRY: parking_lot::RwLock<Option<CollectionRegistry>> =
  parking_lot::RwLock::new(None);
pub fn setup_active_collection_registry(r: CollectionRegistry) -> Option<CollectionRegistry> {
  ACTIVE_REGISTRY.write().replace(r)
}
pub fn global_collection_registry() -> CollectionRegistry {
  ACTIVE_REGISTRY.read().clone().unwrap()
}

impl CollectionRegistry {
  pub fn fork_or_insert_with<K, V, R>(
    &self,
    inserter: impl FnOnce() -> R + Any,
  ) -> impl ReactiveCollection<K, V>
  where
    K: Clone + 'static,
    V: Clone + 'static,
    R: ReactiveCollection<K, V>,
  {
    self.fork_or_insert_with_inner(inserter.type_id(), inserter)
  }

  fn fork_or_insert_with_inner<K, V, R>(
    &self,
    typeid: TypeId,
    inserter: impl FnOnce() -> R,
  ) -> RxCForker<K, V>
  where
    K: Clone + 'static,
    V: Clone + 'static,
    R: ReactiveCollection<K, V>,
  {
    // note, we not using entry api because this call maybe be recursive and cause dead lock
    let registry = self.registry.read_recursive();
    if let Some(collection) = registry.get(&typeid) {
      let collection = collection.downcast_ref::<RxCForker<K, V>>().unwrap();
      collection.clone()
    } else {
      drop(registry);
      let collection = inserter();
      let boxed: Box<dyn DynamicReactiveCollection<K, V>> = Box::new(collection);
      let forker = boxed.into_forker();

      let boxed = Box::new(forker) as Box<dyn Any>;
      let mut registry = self.registry.write();
      registry.insert(typeid, boxed);

      let collection = registry.get(&typeid).unwrap();
      let collection = collection.downcast_ref::<RxCForker<K, V>>().unwrap();
      collection.clone()
    }
  }

  pub fn get_or_create_relation<O, M, R>(
    &self,
    inserter: impl FnOnce() -> R + Any,
  ) -> impl ReactiveOneToManyRelationship<O, M>
  where
    O: LinearIdentification + Clone + 'static,
    M: LinearIdentification + Clone + 'static,
    R: ReactiveCollection<M, O>,
  {
    // note, we not using entry api because this call maybe be recursive and cause dead lock
    let typeid = inserter.type_id();
    let relations = self.relations.read_recursive();
    if let Some(collection) = relations.get(&typeid) {
      let collection = collection
        .downcast_ref::<OneManyRelationForker<O, M>>()
        .unwrap();
      collection.clone()
    } else {
      drop(relations);
      let upstream = self.fork_or_insert_with_inner(typeid, inserter);
      let relation = upstream.into_one_to_many_by_idx_expose_type();

      let boxed = Box::new(relation) as Box<dyn Any>;
      let mut relations = self.relations.write();
      relations.insert(typeid, boxed);

      let relation = relations.get(&typeid).unwrap();
      let relation = relation
        .downcast_ref::<OneManyRelationForker<O, M>>()
        .unwrap();
      relation.clone()
    }
  }
}
