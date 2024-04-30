use std::task::Context;

use fast_hash_collection::FastHashSet;
use futures::task::AtomicWaker;
use reactive::AllocIdx;
use rendiation_scene_core::SceneModelEntity;

#[derive(Default)]
pub struct SelectionSet {
  selected: FastHashSet<AllocIdx<SceneModelEntity>>,
  changed: AtomicWaker,
}

impl<'a> IntoIterator for &'a SelectionSet {
  type Item = &'a AllocIdx<SceneModelEntity>;

  type IntoIter = SelectionSetIterType<'a>;

  fn into_iter(self) -> Self::IntoIter {
    iter(&self.selected)
  }
}

pub type SelectionSetIterType<'a> = impl Iterator<Item = &'a AllocIdx<SceneModelEntity>>;

fn iter(map: &FastHashSet<AllocIdx<SceneModelEntity>>) -> SelectionSetIterType {
  map.iter()
}

impl SelectionSet {
  pub fn setup_waker(&self, cx: &Context) {
    self.changed.register(cx.waker())
  }

  pub fn is_empty(&self) -> bool {
    self.selected.is_empty()
  }

  pub fn select(&mut self, model: AllocIdx<SceneModelEntity>) {
    self.changed.wake();
    self.selected.insert(model);
  }

  pub fn deselect(&mut self, model: AllocIdx<SceneModelEntity>) {
    self.changed.wake();
    self.selected.remove(&model);
  }

  pub fn clear(&mut self) {
    self.changed.wake();
    self.selected.clear();
  }

  pub fn iter_selected(&self) -> impl Iterator<Item = &AllocIdx<SceneModelEntity>> {
    self.selected.iter()
  }
}
