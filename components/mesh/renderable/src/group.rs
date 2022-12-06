use std::ops::Range;

use crate::GPUConsumableMeshBuffer;

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

#[derive(Default, Clone, Debug)]
pub struct MeshGroupsInfo {
  pub groups: Vec<MeshGroup>,
}

impl MeshGroupsInfo {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn push(&mut self, start: usize, count: usize) {
    self.groups.push(MeshGroup { start, count });
  }

  pub fn push_consequent(&mut self, count: usize) {
    let start = self.groups.last().map(|l| l.start + l.count).unwrap_or(0);
    self.groups.push(MeshGroup { start, count });
  }

  pub fn extend_last(&mut self, count: usize) {
    if let Some(last) = &mut self.groups.last_mut() {
      last.count += count;
    } else {
      self.push(0, count);
    }
  }

  pub fn full_from_mesh<T: GPUConsumableMeshBuffer>(mesh: &T) -> Self {
    let mut ranges = MeshGroupsInfo::new();
    ranges.push(0, mesh.draw_count());
    ranges
  }
}

#[derive(Clone)]
pub struct GroupedMesh<T> {
  pub mesh: T,
  pub groups: MeshGroupsInfo,
}

#[derive(Debug, Clone, Copy)]
pub enum MeshDrawGroup {
  Full,
  SubMesh(usize),
}

incremental::clone_self_incremental!(MeshDrawGroup);

impl Default for MeshDrawGroup {
  fn default() -> Self {
    Self::Full
  }
}

impl<T> GroupedMesh<T> {
  pub fn new(mesh: T, groups: MeshGroupsInfo) -> Self {
    Self { mesh, groups }
  }
}

impl<T: GPUConsumableMeshBuffer> GroupedMesh<T> {
  pub fn full(mesh: T) -> Self {
    let groups = MeshGroupsInfo::full_from_mesh(&mesh);
    Self { mesh, groups }
  }

  pub fn get_group(&self, group: MeshDrawGroup) -> MeshGroup {
    match group {
      MeshDrawGroup::Full => self.mesh.get_full_group(),
      MeshDrawGroup::SubMesh(i) => *self.groups.groups.get(i).unwrap(),
    }
  }
}
