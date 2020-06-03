use super::{BuildPrimitive, BVHBounding, FlattenBVHNode};
use rendiation_math_entity::{Axis3, AABB, Sphere};
use rendiation_math::Vec3;
use std::cmp::Ordering;

impl BVHBounding for AABB {
  type AxisType = Axis3;
  fn get_center(&self) -> Vec3<f32> {
    self.center()
  }
  fn from_groups(iter: impl Iterator<Item = Self>) -> Self {
    Self::from_boxes(iter)
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

impl BVHBounding for Sphere {
  type AxisType = Axis3;
  fn get_center(&self) -> Vec3<f32> {
    self.center
  }
  fn from_groups(_iter: impl Iterator<Item = Self>) -> Self {
    todo!()
    // Self::from_boxes(iter)
  }
  fn get_partition_axis(
    _node: &FlattenBVHNode<Self>,
    _build_source: &Vec<BuildPrimitive<Self>>,
    _index_source: &Vec<usize>,
  ) -> Self::AxisType {
    todo!()
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
