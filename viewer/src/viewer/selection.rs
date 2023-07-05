use fast_hash_collection::FastHashSet;

use crate::*;

#[derive(Default)]
pub struct SelectionSet {
  pub selected: FastHashSet<SceneModel>,
}

impl<'a> IntoIterator for &'a SelectionSet {
  type Item = &'a SceneModel;

  type IntoIter = SelectionSetIterType<'a>;

  fn into_iter(self) -> Self::IntoIter {
    iter(&self.selected)
  }
}

type SelectionSetIterType<'a> = impl Iterator<Item = &'a SceneModel>;

fn iter(map: &FastHashSet<SceneModel>) -> SelectionSetIterType {
  map.iter()
}

impl SelectionSet {
  pub fn is_empty(&self) -> bool {
    self.selected.is_empty()
  }

  pub fn select(&mut self, model: &SceneModel) {
    self.selected.insert(model.clone());
  }

  pub fn deselect(&mut self, model: &SceneModel) {
    self.selected.remove(model);
  }

  pub fn clear(&mut self) {
    self.selected.clear();
  }

  pub fn as_renderables(&self) -> impl Iterator<Item = &dyn SceneRenderable> {
    self.selected.iter().map(|m| m as &dyn SceneRenderable)
  }
}
