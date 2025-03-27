use crate::*;

pub(crate) trait ShrinkableAny: Any + Send + Sync {
  fn as_any(&self) -> &dyn Any;
  fn shrink_to_fit(&mut self);
}

pub type RQForker<K, V> = ReactiveQueryFork<BoxedDynReactiveQuery<K, V>, K, V>;

pub type OneManyRelationForker<O, M> =
  ReactiveQueryFork<BoxedDynReactiveOneToManyRelation<O, M>, M, O>;

impl<K, V> ShrinkableAny for RQForker<K, V>
where
  K: CKey,
  V: CValue,
{
  fn as_any(&self) -> &dyn Any {
    self
  }
  fn shrink_to_fit(&mut self) {
    self.request(&mut ReactiveQueryRequest::MemoryShrinkToFit);
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
    self.request(&mut ReactiveQueryRequest::MemoryShrinkToFit);
  }
}

#[derive(Default, Clone)]
pub struct ReactiveQueryRegistry {
  registry: Arc<RwLock<FastHashMap<TypeId, Box<dyn ShrinkableAny>>>>,
  // note, we can not merge these maps because their key is overlapping but the value is not
  // actually same
  index_relation: Arc<RwLock<FastHashMap<TypeId, Box<dyn ShrinkableAny>>>>,
  hash_relation: Arc<RwLock<FastHashMap<TypeId, Box<dyn ShrinkableAny>>>>,
}

static ACTIVE_REGISTRY: parking_lot::RwLock<Option<ReactiveQueryRegistry>> =
  parking_lot::RwLock::new(None);
pub fn setup_active_reactive_query_registry(
  r: ReactiveQueryRegistry,
) -> Option<ReactiveQueryRegistry> {
  ACTIVE_REGISTRY.write().replace(r)
}
pub fn global_reactive_query_registry() -> ReactiveQueryRegistry {
  ACTIVE_REGISTRY.read().clone().unwrap()
}

impl ReactiveQueryRegistry {
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

  pub fn update_and_read_query<K: CKey, V: CValue>(&self, id: TypeId) -> BoxedDynQuery<K, V> {
    let registry = self.registry.read_recursive();
    registry
      .get(&id)
      .unwrap()
      .as_ref()
      .as_any()
      .downcast_ref::<RQForker<K, V>>()
      .unwrap()
      .update_and_read()
  }

  pub fn update_and_read_multi_query<K: CKey, V: CKey>(
    &self,
    id: TypeId,
  ) -> BoxedDynMultiQuery<K, V> {
    let registry = self.registry.read_recursive();
    registry
      .get(&id)
      .unwrap()
      .as_ref()
      .as_any()
      .downcast_ref::<OneManyRelationForker<K, V>>()
      .unwrap()
      .update_and_read()
  }

  pub fn fork_or_insert_with<R>(
    &self,
    inserter: impl FnOnce() -> R + Any,
  ) -> impl ReactiveQuery<Key = R::Key, Value = R::Value> + Clone
  where
    R: ReactiveQuery,
  {
    self.fork_or_insert_with_inner(inserter.type_id(), inserter)
  }

  fn fork_or_insert_with_inner<R>(
    &self,
    typeid: TypeId,
    inserter: impl FnOnce() -> R,
  ) -> RQForker<R::Key, R::Value>
  where
    R: ReactiveQuery,
  {
    // note, we not using entry api because this call maybe be recursive and cause dead lock
    let registry = self.registry.read_recursive();
    if let Some(query) = registry.get(&typeid) {
      let query = query
        .as_ref()
        .as_any()
        .downcast_ref::<RQForker<R::Key, R::Value>>()
        .unwrap();
      query.clone()
    } else {
      drop(registry);
      let query = inserter();
      let boxed: BoxedDynReactiveQuery<R::Key, R::Value> = Box::new(query);
      let forker = boxed.into_static_forker();

      let boxed = Box::new(forker) as Box<dyn ShrinkableAny>;
      let mut registry = self.registry.write();
      registry.insert(typeid, boxed);

      let query = registry.get(&typeid).unwrap();
      let query = query
        .as_ref()
        .as_any()
        .downcast_ref::<RQForker<R::Key, R::Value>>()
        .unwrap();
      query.clone()
    }
  }

  pub fn get_or_create_relation_by_idx<R>(
    &self,
    inserter: impl FnOnce() -> R + Any,
  ) -> impl ReactiveOneToManyRelation + Clone
  where
    R: ReactiveQuery,
    R::Key: LinearIdentification,
    R::Value: LinearIdentification + CKey,
  {
    // note, we not using entry api because this call maybe be recursive and cause dead lock
    let typeid = inserter.type_id();
    let relations = self.index_relation.read_recursive();
    if let Some(query) = relations.get(&typeid) {
      let query = query
        .as_ref()
        .as_any()
        .downcast_ref::<OneManyRelationForker<R::Value, R::Key>>()
        .unwrap();
      query.clone()
    } else {
      drop(relations);
      let upstream = self.fork_or_insert_with_inner(typeid, inserter);
      let relation = upstream.into_one_to_many_by_idx();
      let relation = Box::new(relation) as BoxedDynReactiveOneToManyRelation<R::Value, R::Key>;
      let relation = ReactiveQueryFork::new(relation, true);

      let boxed = Box::new(relation) as Box<dyn ShrinkableAny>;
      let mut relations = self.index_relation.write();
      relations.insert(typeid, boxed);

      let relation = relations.get(&typeid).unwrap();
      let relation = relation
        .as_ref()
        .as_any()
        .downcast_ref::<OneManyRelationForker<R::Value, R::Key>>()
        .unwrap();
      relation.clone()
    }
  }

  pub fn get_or_create_relation_by_hash<R>(
    &self,
    inserter: impl FnOnce() -> R + Any,
  ) -> impl ReactiveOneToManyRelation<One = R::Value, Many = R::Key> + Clone
  where
    R: ReactiveQuery,
    R::Value: CKey,
  {
    // note, we not using entry api because this call maybe be recursive and cause dead lock
    let typeid = inserter.type_id();
    let relations = self.hash_relation.read_recursive();
    if let Some(query) = relations.get(&typeid) {
      let query = query
        .as_ref()
        .as_any()
        .downcast_ref::<OneManyRelationForker<R::Value, R::Key>>()
        .unwrap();
      query.clone()
    } else {
      drop(relations);
      let upstream = self.fork_or_insert_with_inner(typeid, inserter);
      let relation = upstream.into_one_to_many_by_hash();
      let relation = Box::new(relation) as BoxedDynReactiveOneToManyRelation<R::Value, R::Key>;
      let relation = ReactiveQueryFork::new(relation, true);

      let boxed = Box::new(relation) as Box<dyn ShrinkableAny>;
      let mut relations = self.hash_relation.write();
      relations.insert(typeid, boxed);

      let relation = relations.get(&typeid).unwrap();
      let relation = relation
        .as_ref()
        .as_any()
        .downcast_ref::<OneManyRelationForker<R::Value, R::Key>>()
        .unwrap();
      relation.clone()
    }
  }
}
