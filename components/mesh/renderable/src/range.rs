use crate::mesh::AbstractMesh;

#[derive(Copy, Clone, Debug)]
pub struct MeshRange {
  pub start: usize,
  pub count: usize,
}

pub struct MeshRangesInfo {
  pub ranges: Vec<MeshRange>,
}

impl Default for MeshRangesInfo {
  fn default() -> Self {
    Self::new()
  }
}

impl MeshRangesInfo {
  pub fn new() -> Self {
    Self { ranges: Vec::new() }
  }

  pub fn push(&mut self, start: usize, count: usize) {
    self.ranges.push(MeshRange { start, count });
  }

  pub fn full_range<T: AbstractMesh>(mesh: &T) -> Self {
    let mut ranges = MeshRangesInfo::new();
    ranges.push(0, mesh.draw_count());
    ranges
  }
}
