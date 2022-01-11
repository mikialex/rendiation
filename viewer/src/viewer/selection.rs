use std::collections::HashMap;

use rendiation_algebra::Vec2;
use rendiation_renderable_mesh::mesh::MeshBufferIntersectConfig;

use crate::{MeshModel, MeshModelImpl, Scene};

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
  pub selected: HashMap<usize, MeshModel>,
}

impl<'a> IntoIterator for &'a mut SelectionSet {
  type Item = &'a mut MeshModel;

  type IntoIter = SelectionSetIterMutType<'a>;

  fn into_iter(self) -> Self::IntoIter {
    mut_iter(&mut self.selected)
  }
}

type SelectionSetIterMutType<'a> = impl Iterator<Item = &'a mut MeshModel>;

fn mut_iter(map: &mut HashMap<*const MeshModelImpl, MeshModel>) -> SelectionSetIterMutType {
  map.iter_mut().map(|(_, m)| m)
}

impl<'a> IntoIterator for &'a SelectionSet {
  type Item = &'a MeshModel;

  type IntoIter = SelectionSetIterType<'a>;

  fn into_iter(self) -> Self::IntoIter {
    iter(&self.selected)
  }
}

type SelectionSetIterType<'a> = impl Iterator<Item = &'a MeshModel>;

fn iter(map: &HashMap<*const MeshModelImpl, MeshModel>) -> SelectionSetIterType {
  map.iter().map(|(_, m)| m)
}

impl SelectionSet {
  pub fn is_empty(&self) -> bool {
    self.selected.is_empty()
  }

  pub fn select(&mut self, model: &MeshModel) {
    self.selected.insert(model.inner.as_ptr(), model.clone());
  }

  pub fn deselect(&mut self, model: &MeshModel) {
    self.selected.remove(&(model.inner.as_ptr() as *const _));
  }

  pub fn clear(&mut self) {
    self.selected.clear();
  }
}
