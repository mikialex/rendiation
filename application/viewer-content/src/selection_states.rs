use crate::*;

#[derive(Default)]
pub struct ViewerSelectionStates {
  pub selected_model: ViewerModelSelectionSet,
  pub selected_dir_light: Option<EntityHandle<DirectionalLightEntity>>,
  pub selected_spot_light: Option<EntityHandle<SpotLightEntity>>,
  pub selected_point_light: Option<EntityHandle<PointLightEntity>>,
}

#[derive(Default)]
pub struct ViewerModelSelectionSet {
  selected_models: FastHashSet<EntityHandle<SceneModelEntity>>,
}

impl ViewerModelSelectionSet {
  pub fn iter_selected(
    &self,
  ) -> impl Iterator<Item = EntityHandle<SceneModelEntity>> + Clone + 'static {
    // todo, improve
    self
      .selected_models
      .iter()
      .copied()
      .collect::<Vec<_>>()
      .into_iter()
  }
  pub fn remove_select_if(&mut self, f: impl Fn(EntityHandle<SceneModelEntity>) -> bool) {
    self.selected_models.retain(|m| !f(*m));
  }

  pub fn has_selected(&self) -> bool {
    !self.selected_models.is_empty()
  }

  pub fn add_select(&mut self, model: EntityHandle<SceneModelEntity>) {
    self.selected_models.insert(model);
  }

  pub fn clear(&mut self) {
    self.selected_models.clear();
  }

  pub fn if_single(&self) -> Option<EntityHandle<SceneModelEntity>> {
    if self.selected_models.len() == 1 {
      self.selected_models.iter().next().copied()
    } else {
      None
    }
  }
}
