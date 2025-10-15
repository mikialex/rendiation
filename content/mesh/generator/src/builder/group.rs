use crate::*;

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct MeshGroup {
  pub start: usize,
  pub count: usize,
}

impl From<MeshGroup> for Range<u32> {
  fn from(range: MeshGroup) -> Self {
    range.start as u32..(range.start + range.count) as u32
  }
}

#[derive(Default, Clone, Debug, Hash, PartialEq, Eq)]
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
}

#[derive(Clone, Default)]
pub struct GroupedMesh<T> {
  pub mesh: T,
  pub groups: MeshGroupsInfo,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Default)]
pub enum MeshDrawGroup {
  #[default]
  Full,
  SubMesh(usize),
}

impl<T> GroupedMesh<T> {
  pub fn new(mesh: T, groups: MeshGroupsInfo) -> Self {
    Self { mesh, groups }
  }
}
