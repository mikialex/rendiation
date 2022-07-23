use std::collections::HashMap;

use crate::*;

type Selected = Box<dyn SceneRenderableShareable>;

#[derive(Default)]
pub struct SelectionSet {
  pub selected: HashMap<usize, Selected>,
}

impl<'a> IntoIterator for &'a mut SelectionSet {
  type Item = &'a mut dyn SceneRenderable;

  type IntoIter = SelectionSetIterMutType<'a>;

  fn into_iter(self) -> Self::IntoIter {
    mut_iter(&mut self.selected)
  }
}

type SelectionSetIterMutType<'a> = impl Iterator<Item = &'a mut dyn SceneRenderable>;

fn mut_iter(map: &mut HashMap<usize, Selected>) -> SelectionSetIterMutType {
  map.iter_mut().map(|(_, m)| m.as_mut().as_renderable_mut())
}

impl<'a> IntoIterator for &'a SelectionSet {
  type Item = &'a dyn SceneRenderable;

  type IntoIter = SelectionSetIterType<'a>;

  fn into_iter(self) -> Self::IntoIter {
    iter(&self.selected)
  }
}

type SelectionSetIterType<'a> = impl Iterator<Item = &'a dyn SceneRenderable>;

fn iter(map: &HashMap<usize, Selected>) -> SelectionSetIterType {
  map.iter().map(|(_, m)| m.as_ref().as_renderable())
}

impl SelectionSet {
  pub fn is_empty(&self) -> bool {
    self.selected.is_empty()
  }

  pub fn select(&mut self, model: &dyn SceneRenderableShareable) {
    self.selected.insert(model.id(), model.clone_boxed());
  }

  pub fn deselect(&mut self, model: &dyn SceneRenderableShareable) {
    self.selected.remove(&model.id());
  }

  pub fn clear(&mut self) {
    self.selected.clear();
  }
}
