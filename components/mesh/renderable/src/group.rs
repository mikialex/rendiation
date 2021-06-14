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
  pub ranges: Vec<MeshGroup>,
}

impl Default for MeshGroupsInfo {
  fn default() -> Self {
    Self::new()
  }
}

impl MeshGroupsInfo {
  pub fn new() -> Self {
    Self { ranges: Vec::new() }
  }

  pub fn push(&mut self, start: usize, count: usize) {
    self.ranges.push(MeshGroup { start, count });
  }

  pub fn full_range<T: AbstractMesh>(mesh: &T) -> Self {
    let mut ranges = MeshGroupsInfo::new();
    ranges.push(0, mesh.draw_count());
    ranges
  }
}
