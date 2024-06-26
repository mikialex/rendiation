use crate::*;

pub(crate) trait ShrinkableAny: Any + Send + Sync {
  fn as_any(&self) -> &dyn Any;
  fn shrink_to_fit(&mut self);
}

pub type RxCForker<K, V> = ReactiveKVMapFork<Box<dyn DynReactiveCollection<K, V>>, K, V>;

pub type OneManyRelationForker<O, M> =
  ReactiveKVMapFork<Box<dyn DynReactiveOneToManyRelation<O, M>>, M, O>;

impl<K, V> ShrinkableAny for RxCForker<K, V>
where
  K: CKey,
  V: CValue,
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
  O: CKey,
  M: CKey,
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
  ) -> impl ReactiveCollection<K, V> + Clone
  where
    K: CKey,
    V: CValue,
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
    K: CKey,
    V: CValue,
    R: ReactiveCollection<K, V>,
  {
    // note, we not using entry api because this call maybe be recursive and cause dead lock
    let registry = self.registry.read_recursive();
    if let Some(collection) = registry.get(&typeid) {
      let collection = collection
        .as_ref()
        .as_any()
        .downcast_ref::<RxCForker<K, V>>()
        .unwrap();
      collection.clone()
    } else {
      drop(registry);
      let collection = inserter();
      let boxed: Box<dyn DynReactiveCollection<K, V>> = Box::new(collection);
      let forker = boxed.into_forker();

      let boxed = Box::new(forker) as Box<dyn ShrinkableAny>;
      let mut registry = self.registry.write();
      registry.insert(typeid, boxed);

      let collection = registry.get(&typeid).unwrap();
      let collection = collection
        .as_ref()
        .as_any()
        .downcast_ref::<RxCForker<K, V>>()
        .unwrap();
      collection.clone()
    }
  }

  pub fn get_or_create_relation_by_idx<O, M, R>(
    &self,
    inserter: impl FnOnce() -> R + Any,
  ) -> impl ReactiveOneToManyRelation<O, M> + Clone
  where
    O: LinearIdentification + CKey,
    M: LinearIdentification + CKey,
    R: ReactiveCollection<M, O>,
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
      let relation = Box::new(relation) as Box<dyn DynReactiveOneToManyRelation<O, M>>;
      let relation = ReactiveKVMapFork::new(relation, true);

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
  ) -> impl ReactiveOneToManyRelation<O, M> + Clone
  where
    O: CKey,
    M: CKey,
    R: ReactiveCollection<M, O>,
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
      let relation = Box::new(relation) as Box<dyn DynReactiveOneToManyRelation<O, M>>;
      let relation = ReactiveKVMapFork::new(relation, true);

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
