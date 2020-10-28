use super::{BVHBounding, BuildPrimitive, FlattenBVHNode, SAHBounding};
use rendiation_math::Vec3;
use rendiation_math_entity::{Axis3, Box3};
use std::{cmp::Ordering, ops::Range};

impl BVHBounding for Box3 {
  type AxisType = Axis3;

  fn get_partition_axis(
    node: &FlattenBVHNode<Self>,
    _build_source: &Vec<BuildPrimitive<Self>>,
    _index_source: &Vec<usize>,
  ) -> Self::AxisType {
    node.bounding.longest_axis().0
  }

  fn compare(
    self_p: &BuildPrimitive<Self>,
    other: &BuildPrimitive<Self>,
    axis: Self::AxisType,
  ) -> Ordering {
    match axis {
      Axis3::X => self_p.center.x.partial_cmp(&other.center.x).unwrap(),
      Axis3::Y => self_p.center.y.partial_cmp(&other.center.y).unwrap(),
      Axis3::Z => self_p.center.z.partial_cmp(&other.center.z).unwrap(),
    }
  }
}

impl SAHBounding for Box3 {
  fn get_unit_range_by_axis(&self, axis: Axis3) -> Range<f32> {
    match axis {
      Axis3::X => self.min.x..self.max.x,
      Axis3::Y => self.min.y..self.max.y,
      Axis3::Z => self.min.z..self.max.z,
    }
  }

  fn get_unit_from_center_by_axis(center: &Vec3<f32>, axis: Axis3) -> f32 {
    match axis {
      Axis3::X => center.x,
      Axis3::Y => center.y,
      Axis3::Z => center.z,
    }
  }

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
}
