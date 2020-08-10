use super::{BVHBounding, BuildPrimitive, FlattenBVHNode};
use rendiation_math::Vec3;
use rendiation_math_entity::{Axis3, Box3};
use std::cmp::Ordering;

impl BVHBounding for Box3 {
  type AxisType = Axis3;
  type CenterType = Vec3<f32>;

  fn get_center(&self) -> Vec3<f32> {
    self.center()
  }

  fn get_partition_axis(
    node: &FlattenBVHNode<Self>,
    _build_source: &Vec<BuildPrimitive<Self>>,
    _index_source: &Vec<usize>,
  ) -> Self::AxisType {
    node.bounding.longest_axis().0
  }

  fn compare(
    self_p: &BuildPrimitive<Self>,
    axis: Self::AxisType,
    other: &BuildPrimitive<Self>,
  ) -> Ordering {
    match axis {
      Axis3::X => self_p.center.x.partial_cmp(&other.center.x).unwrap(),
      Axis3::Y => self_p.center.y.partial_cmp(&other.center.y).unwrap(),
      Axis3::Z => self_p.center.z.partial_cmp(&other.center.z).unwrap(),
    }
  }
}
