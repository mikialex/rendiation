use std::ops::Range;

use crate::mesh::AbstractMesh;

#[derive(Copy, Clone, Debug)]
pub struct MeshGroup {
  pub start: usize,
  pub count: usize,
}

impl From<MeshGroup> for Range<u32> {
  fn from(range: MeshGroup) -> Self {
    range.start as u32..(range.start + range.count) as u32
  }
}

pub struct MeshGroupsInfo {
  pub groups: Vec<MeshGroup>,
}

impl Default for MeshGroupsInfo {
  fn default() -> Self {
    Self::new()
  }
}

impl MeshGroupsInfo {
  pub fn new() -> Self {
    Self { groups: Vec::new() }
  }

  pub fn push(&mut self, start: usize, count: usize) {
    self.groups.push(MeshGroup { start, count });
  }

  pub fn full<T: AbstractMesh>(mesh: &T) -> Self {
    let mut ranges = MeshGroupsInfo::new();
    ranges.push(0, mesh.draw_count());
    ranges
  }
}

pub struct GroupedMesh<T> {
  pub mesh: T,
  pub groups: MeshGroupsInfo,
}

#[derive(Debug, Clone, Copy)]
pub enum MeshDrawGroup {
  Full,
  SubMesh(usize),
}

impl Default for MeshDrawGroup {
  fn default() -> Self {
    Self::Full
  }
}

impl<T: AbstractMesh> GroupedMesh<T> {
  pub fn new(mesh: T, groups: MeshGroupsInfo) -> Self {
    Self { mesh, groups }
  }
  pub fn full(mesh: T) -> Self {
    let groups = MeshGroupsInfo::full(&mesh);
    Self { mesh, groups }
  }

  pub fn get_group(&self, group: MeshDrawGroup) -> MeshGroup {
    match group {
      MeshDrawGroup::Full => self.mesh.get_full_group(),
      MeshDrawGroup::SubMesh(i) => *self.groups.groups.get(i).unwrap(),
    }
  }
}
