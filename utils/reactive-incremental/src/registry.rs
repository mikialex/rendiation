use std::any::{Any, TypeId};

use fast_hash_collection::FastHashMap;
use parking_lot::RwLock;

use crate::*;

type Forker<K, V> = ReactiveKVMapFork<Box<dyn DynamicReactiveCollection<K, V>>, K, V>;

#[derive(Default)]
pub struct CollectionRegistry {
  registry: RwLock<FastHashMap<TypeId, Box<dyn Any>>>,
}

impl CollectionRegistry {
  pub fn fork_or_insert_with<K, V, R>(
    &self,
    ty: impl Any,
    inserter: impl FnOnce() -> R,
  ) -> impl ReactiveCollection<K, V>
  where
    K: Clone + 'static,
    V: Clone + 'static,
    R: ReactiveCollection<K, V>,
  {
    // note, we not using entry api because this call maybe be recursive and cause dead lock
    let type_id = ty.type_id();
    let typeid = type_id;
    let registry = self.registry.read_recursive();
    if let Some(collection) = registry.get(&typeid) {
      let collection = collection.downcast_ref::<Forker<K, V>>().unwrap();
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
      let collection = collection.downcast_ref::<Forker<K, V>>().unwrap();
      collection.clone()
    }
  }
}
