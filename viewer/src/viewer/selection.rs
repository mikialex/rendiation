use std::task::Context;

use fast_hash_collection::FastHashSet;
use futures::task::AtomicWaker;

use crate::*;

#[derive(Default)]
pub struct SelectionSet {
  selected: FastHashSet<SceneModel>,
  changed: AtomicWaker,
}

impl<'a> IntoIterator for &'a SelectionSet {
  type Item = &'a SceneModel;

  type IntoIter = SelectionSetIterType<'a>;

  fn into_iter(self) -> Self::IntoIter {
    iter(&self.selected)
  }
}

pub type SelectionSetIterType<'a> = impl Iterator<Item = &'a SceneModel>;

fn iter(map: &FastHashSet<SceneModel>) -> SelectionSetIterType {
  map.iter()
}

impl SelectionSet {
  pub fn setup_waker(&self, cx: &Context) {
    self.changed.register(cx.waker())
  }

  pub fn is_empty(&self) -> bool {
    self.selected.is_empty()
  }

  pub fn select(&mut self, model: &SceneModel) {
    self.changed.wake();
    self.selected.insert(model.clone());
  }

  pub fn deselect(&mut self, model: &SceneModel) {
    self.changed.wake();
    self.selected.remove(model);
  }

  pub fn clear(&mut self) {
    self.changed.wake();
    self.selected.clear();
  }

  pub fn iter_selected(&self) -> impl Iterator<Item = &SceneModel> {
    self.selected.iter()
  }

  pub fn iter_renderables(&self) -> impl Iterator<Item = &dyn SceneRenderable> {
    self.selected.iter().map(|m| m as &dyn SceneRenderable)
  }
}
