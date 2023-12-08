use std::{
  any::{Any, TypeId},
  sync::Arc,
};

use fast_hash_collection::FastHashMap;
use parking_lot::RwLock;

use crate::*;

pub(crate) trait ShrinkableAny: Any + Send + Sync {
  fn as_any(&self) -> &dyn Any;
  fn shrink_to_fit(&mut self);
}

/// https://en.wikipedia.org/wiki/Plane_(Dungeons_%26_Dragons)
#[derive(Default)]
pub struct PLANE {
  storages: FastHashMap<TypeId, Box<dyn ShrinkableAny>>,
}

impl<T: IncrementalBase> ShrinkableAny for IncrementalSignalStorage<T> {
  fn as_any(&self) -> &dyn Any {
    self
  }

  fn shrink_to_fit(&mut self) {
    self.inner.data.write().shrink_to_fit();
    self.inner.sub_watchers.write().shrink_to_fit();
  }
}

static ACTIVE_PLANE: parking_lot::RwLock<Option<PLANE>> = parking_lot::RwLock::new(None);
pub fn setup_active_plane(sg: PLANE) -> Option<PLANE> {
  ACTIVE_PLANE.write().replace(sg)
}

pub fn access_storage_of<T: IncrementalBase, R>(
  acc: impl FnOnce(&IncrementalSignalStorage<T>) -> R,
) -> R {
  let id = TypeId::of::<T>();

  // not add write lock first if the storage exists
  let try_read_storages = ACTIVE_PLANE.read();
  let storages = try_read_storages
    .as_ref()
    .expect("global storage group not specified");
  if let Some(storage) = storages.storages.get(&id) {
    let storage = storage
      .as_ref()
      .as_any()
      .downcast_ref::<IncrementalSignalStorage<T>>()
      .unwrap();
    acc(storage)
  } else {
    drop(try_read_storages);
    let mut storages = ACTIVE_PLANE.write();
    let storages = storages
      .as_mut()
      .expect("global storage group not specified");
    let storage = storages
      .storages
      .entry(id)
      .or_insert_with(|| Box::<IncrementalSignalStorage<T>>::default());
    let storage = storage
      .as_ref()
      .as_any()
      .downcast_ref::<IncrementalSignalStorage<T>>()
      .unwrap();
    acc(storage)
  }
}
pub fn storage_of<T: IncrementalBase>() -> IncrementalSignalStorage<T> {
  access_storage_of(|s| s.clone())
}

pub type RxCForkerWithPrevious<K, V> = ReactiveKVMapFork<
  Box<dyn DynamicReactiveCollectionWithPrevious<K, V>>,
  CollectionChangesWithPrevious<K, V>,
  K,
  V,
>;

pub type RxCForker<K, V> =
  ReactiveKVMapFork<Box<dyn DynamicReactiveCollection<K, V>>, CollectionChanges<K, V>, K, V>;

pub type OneManyRelationForker<O, M> = ReactiveKVMapFork<
  Box<dyn DynamicReactiveOneToManyRelationship<O, M>>,
  CollectionChangesWithPrevious<M, O>,
  M,
  O,
>;

impl<K, V> ShrinkableAny for RxCForkerWithPrevious<K, V>
where
  K: Send + Sync + Clone + Eq + std::hash::Hash + 'static,
  V: Send + Sync + Clone + 'static,
{
  fn as_any(&self) -> &dyn Any {
    self
  }
  fn shrink_to_fit(&mut self) {
    self.extra_request(&mut ExtraCollectionOperation::MemoryShrinkToFit);
  }
}
impl<O, M> ShrinkableAny for OneManyRelationForker<O, M>
where
  O: Send + Sync + Clone + 'static,
  M: Send + Sync + Clone + Eq + std::hash::Hash + 'static,
{
  fn as_any(&self) -> &dyn Any {
    self
  }
  fn shrink_to_fit(&mut self) {
    self.extra_request(&mut ExtraCollectionOperation::MemoryShrinkToFit);
  }
}

#[derive(Default, Clone)]
pub struct CollectionRegistry {
  registry: Arc<RwLock<FastHashMap<TypeId, Box<dyn ShrinkableAny>>>>,
  // note, we can not merge these maps because their key is overlapping but the value is not
  // actually same
  index_relation: Arc<RwLock<FastHashMap<TypeId, Box<dyn ShrinkableAny>>>>,
  hash_relation: Arc<RwLock<FastHashMap<TypeId, Box<dyn ShrinkableAny>>>>,
}

static ACTIVE_REGISTRY: parking_lot::RwLock<Option<CollectionRegistry>> =
  parking_lot::RwLock::new(None);
pub fn setup_active_collection_registry(r: CollectionRegistry) -> Option<CollectionRegistry> {
  ACTIVE_REGISTRY.write().replace(r)
}
pub fn global_collection_registry() -> CollectionRegistry {
  ACTIVE_REGISTRY.read().clone().unwrap()
}

impl CollectionRegistry {
  pub fn shrink_to_fit_all(&self) {
    let mut registry = self.registry.write();
    for v in registry.values_mut() {
      v.shrink_to_fit();
    }
    let mut registry = self.hash_relation.write();
    for v in registry.values_mut() {
      v.shrink_to_fit();
    }
    let mut registry = self.index_relation.write();
    for v in registry.values_mut() {
      v.shrink_to_fit();
    }
  }

  pub fn fork_or_insert_with<K, V, R>(
    &self,
    inserter: impl FnOnce() -> R + Any,
  ) -> impl ReactiveCollectionWithPrevious<K, V> + Clone
  where
    K: Clone + Send + Sync + Eq + std::hash::Hash + 'static,
    V: Clone + Send + Sync + 'static,
    R: ReactiveCollectionWithPrevious<K, V>,
  {
    self.fork_or_insert_with_inner(inserter.type_id(), inserter)
  }

  fn fork_or_insert_with_inner<K, V, R>(
    &self,
    typeid: TypeId,
    inserter: impl FnOnce() -> R,
  ) -> RxCForkerWithPrevious<K, V>
  where
    K: Clone + Send + Sync + Eq + std::hash::Hash + 'static,
    V: Clone + Send + Sync + 'static,
    R: ReactiveCollectionWithPrevious<K, V>,
  {
    // note, we not using entry api because this call maybe be recursive and cause dead lock
    let registry = self.registry.read_recursive();
    if let Some(collection) = registry.get(&typeid) {
      let collection = collection
        .as_ref()
        .as_any()
        .downcast_ref::<RxCForkerWithPrevious<K, V>>()
        .unwrap();
      collection.clone()
    } else {
      drop(registry);
      let collection = inserter();
      let boxed: Box<dyn DynamicReactiveCollectionWithPrevious<K, V>> = Box::new(collection);
      let forker = boxed.into_forker();

      let boxed = Box::new(forker) as Box<dyn ShrinkableAny>;
      let mut registry = self.registry.write();
      registry.insert(typeid, boxed);

      let collection = registry.get(&typeid).unwrap();
      let collection = collection
        .as_ref()
        .as_any()
        .downcast_ref::<RxCForkerWithPrevious<K, V>>()
        .unwrap();
      collection.clone()
    }
  }

  pub fn get_or_create_relation_by_idx<O, M, R>(
    &self,
    inserter: impl FnOnce() -> R + Any,
  ) -> impl ReactiveOneToManyRelationship<O, M> + Clone
  where
    O: LinearIdentification + Clone + Send + Sync + 'static,
    M: LinearIdentification + Eq + std::hash::Hash + Clone + Send + Sync + 'static,
    R: ReactiveCollectionWithPrevious<M, O>,
  {
    // note, we not using entry api because this call maybe be recursive and cause dead lock
    let typeid = inserter.type_id();
    let relations = self.index_relation.read_recursive();
    if let Some(collection) = relations.get(&typeid) {
      let collection = collection
        .as_ref()
        .as_any()
        .downcast_ref::<OneManyRelationForker<O, M>>()
        .unwrap();
      collection.clone()
    } else {
      drop(relations);
      let upstream = self.fork_or_insert_with_inner(typeid, inserter);
      let relation = upstream.into_one_to_many_by_idx_expose_type();
      let relation = Box::new(relation) as Box<dyn DynamicReactiveOneToManyRelationship<O, M>>;
      let relation = BufferedCollection::new(ReactiveKVMapForkImpl::new(relation));

      let boxed = Box::new(relation) as Box<dyn ShrinkableAny>;
      let mut relations = self.index_relation.write();
      relations.insert(typeid, boxed);

      let relation = relations.get(&typeid).unwrap();
      let relation = relation
        .as_ref()
        .as_any()
        .downcast_ref::<OneManyRelationForker<O, M>>()
        .unwrap();
      relation.clone()
    }
  }

  pub fn get_or_create_relation_by_hash<O, M, R>(
    &self,
    inserter: impl FnOnce() -> R + Any,
  ) -> impl ReactiveOneToManyRelationship<O, M> + Clone
  where
    O: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
    M: std::hash::Hash + Eq + Clone + Send + Sync + 'static,
    R: ReactiveCollectionWithPrevious<M, O>,
  {
    // note, we not using entry api because this call maybe be recursive and cause dead lock
    let typeid = inserter.type_id();
    let relations = self.hash_relation.read_recursive();
    if let Some(collection) = relations.get(&typeid) {
      let collection = collection
        .as_ref()
        .as_any()
        .downcast_ref::<OneManyRelationForker<O, M>>()
        .unwrap();
      collection.clone()
    } else {
      drop(relations);
      let upstream = self.fork_or_insert_with_inner(typeid, inserter);
      let relation = upstream.into_one_to_many_by_hash_expose_type();
      let relation = Box::new(relation) as Box<dyn DynamicReactiveOneToManyRelationship<O, M>>;
      let relation = BufferedCollection::new(ReactiveKVMapForkImpl::new(relation));

      let boxed = Box::new(relation) as Box<dyn ShrinkableAny>;
      let mut relations = self.hash_relation.write();
      relations.insert(typeid, boxed);

      let relation = relations.get(&typeid).unwrap();
      let relation = relation
        .as_ref()
        .as_any()
        .downcast_ref::<OneManyRelationForker<O, M>>()
        .unwrap();
      relation.clone()
    }
  }
}
