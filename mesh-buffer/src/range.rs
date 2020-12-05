use crate::geometry::AnyGeometry;

#[derive(Copy, Clone, Debug)]
pub struct GeometryRange {
  pub start: usize,
  pub count: usize,
}

pub struct GeometryRangesInfo {
  pub ranges: Vec<GeometryRange>,
}

impl GeometryRangesInfo {
  pub fn new() -> Self {
    Self { ranges: Vec::new() }
  }

  pub fn push(&mut self, start: usize, count: usize) {
    self.ranges.push(GeometryRange { start, count });
  }

  pub fn full_range<T: AnyGeometry>(geometry: &T) -> Self {
    let mut ranges = GeometryRangesInfo::new();
    ranges.push(0, geometry.draw_count());
    ranges
  }
}
