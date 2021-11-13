use std::collections::HashSet;

use crate::MeshModel;

#[derive(Default)]
pub struct SelectionSet {
  selected: HashSet<MeshModel>,
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
