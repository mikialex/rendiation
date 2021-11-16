use std::collections::HashSet;

use rendiation_algebra::Vec2;
use rendiation_renderable_mesh::mesh::MeshBufferIntersectConfig;

use crate::{MeshModel, Scene};

#[derive(Default)]
pub struct Picker {
  pub config: MeshBufferIntersectConfig,
}

impl Picker {
  pub fn pick_new(
    &self,
    scene: &Scene,
    selections: &mut SelectionSet,
    normalized_position: Vec2<f32>,
  ) {
    selections.clear();
    if let Some(nearest) = scene.pick_nearest(normalized_position, &self.config) {
      selections.select(nearest);
    }
  }
}

#[derive(Default)]
pub struct SelectionSet {
  pub selected: HashSet<MeshModel>,
}

impl SelectionSet {
  pub fn select(&mut self, model: &MeshModel) {
    self.selected.insert(model.clone());
  }

  pub fn deselect(&mut self, model: &MeshModel) {
    self.selected.remove(model);
  }

  pub fn clear(&mut self) {
    self.selected.clear();
  }
}
