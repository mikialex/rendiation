use crate::*;

#[derive(Clone)]
pub struct DatabaseEntityReverseReference {
  mutation_watcher: DatabaseMutationWatch,
  entity_rev_refs: Arc<RwLock<StreamMap<ComponentId, Box<dyn Any + Send + Sync>>>>,
}

impl DatabaseEntityReverseReference {
  pub fn new(mutation_watcher: DatabaseMutationWatch) -> Self {
    Self {
      mutation_watcher,
      entity_rev_refs: Default::default(),
    }
  }

  pub fn watch_inv_ref<S: ForeignKeySemantic>(
    &self,
  ) -> Box<dyn ReactiveOneToManyRelationship<u32, u32>> {
    self.watch_inv_ref_dyn(S::component_id(), S::Entity::entity_id())
  }

  pub fn watch_inv_ref_dyn(
    &self,
    semantic_id: ComponentId,
    entity_id: EntityId,
  ) -> Box<dyn ReactiveOneToManyRelationship<u32, u32>> {
    if let Some(refs) = self.entity_rev_refs.read().get(&semantic_id) {
      return Box::new(
        refs
          .downcast_ref::<OneManyRelationForker<u32, u32>>()
          .unwrap()
          .clone(),
      );
    }

    let watcher = self
      .mutation_watcher
      .watch_dyn_foreign_key(semantic_id, entity_id)
      .collective_filter_map(|v| v)
      .into_boxed()
      .into_one_to_many_by_idx_expose_type()
      .into_static_forker();

    self
      .entity_rev_refs
      .write()
      .insert(semantic_id, Box::new(watcher));

    self.watch_inv_ref_dyn(semantic_id, entity_id)
  }
}
