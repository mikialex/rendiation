use crate::utils::BuildPrimitive;

use super::{BVHBounding, BalanceTreeBounding, SAHBounding};
use rendiation_algebra::Vec3;
use rendiation_geometry::{Axis3, Box3};
use std::{cmp::Ordering, ops::Range};

impl BVHBounding for Box3 {
  type AxisType = Axis3;

  #[inline(always)]
  fn get_partition_axis(&self) -> Self::AxisType {
    self.longest_axis().0
  }
}

impl BalanceTreeBounding for Box3 {
  fn median_partition_at_axis(
    range: Range<usize>,
    build_source: &[BuildPrimitive<Self>],
    index_source: &mut Vec<usize>,
    axis: Self::AxisType,
  ) {
    let range_middle = (range.end - range.start) / 2;
    if range_middle == 0 {
      return;
    }
    let ranged_index = index_source.get_mut(range).unwrap();
    use Ordering::*;
    match axis {
      Axis3::X => ranged_index.select_nth_unstable_by(range_middle, |&a, &b| unsafe {
        let bp_a = build_source.get_unchecked(a);
        let bp_b = build_source.get_unchecked(b);
        bp_a.center.x.partial_cmp(&bp_b.center.x).unwrap_or(Less)
      }),
      Axis3::Y => ranged_index.select_nth_unstable_by(range_middle, |&a, &b| unsafe {
        let bp_a = build_source.get_unchecked(a);
        let bp_b = build_source.get_unchecked(b);
        bp_a.center.y.partial_cmp(&bp_b.center.y).unwrap_or(Less)
      }),
      Axis3::Z => ranged_index.select_nth_unstable_by(range_middle, |&a, &b| unsafe {
        let bp_a = build_source.get_unchecked(a);
        let bp_b = build_source.get_unchecked(b);
        bp_a.center.z.partial_cmp(&bp_b.center.z).unwrap_or(Less)
      }),
    };
  }
}

impl SAHBounding for Box3 {
  #[inline(always)]
  fn get_unit_range_by_axis(&self, axis: Axis3) -> Range<f32> {
    match axis {
      Axis3::X => self.min.x..self.max.x,
      Axis3::Y => self.min.y..self.max.y,
      Axis3::Z => self.min.z..self.max.z,
    }
  }

  #[inline(always)]
  fn get_unit_from_center_by_axis(center: &Vec3<f32>, axis: Axis3) -> f32 {
    match axis {
      Axis3::X => center.x,
      Axis3::Y => center.y,
      Axis3::Z => center.z,
    }
  }

  #[inline(always)]
  fn get_surface_heuristic(&self) -> f32 {
    let x_expand = self.max.x - self.min.x;
    let y_expand = self.max.y - self.min.y;
    let z_expand = self.max.z - self.min.z;
    if x_expand < 0.0 || y_expand < 0.0 || z_expand < 0.0 {
      0.0
    } else {
      x_expand * y_expand + x_expand * z_expand + y_expand * z_expand
    }
  }

  #[inline(always)]
  fn empty() -> Self {
    Self::empty()
  }
  #[inline(always)]
  fn union(&mut self, other: Self) {
    self.union(other)
  }
}
