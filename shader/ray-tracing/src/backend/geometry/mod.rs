mod compute;
mod native;
// pub use compute::*;
// pub use native::*;

use crate::*;

pub trait GPUAccelerationStructureProvider {
  fn traverse(
    &self,
    intersect: &dyn Fn(),
    any_hit: &dyn Fn(Node<WorldHitInfo>),
    nearest_hit: &dyn Fn(Node<WorldHitInfo>),
    missing: &dyn Fn(),
  );
}

pub trait GPURayTracingAccelerationStructureDeviceProvider {
  fn create_top_level_acceleration_structure(
    &self,
    boxes: &[Vec3<f32>],
  ) -> Box<dyn GPUAccelerationStructureProvider>;
  fn create_bottom_level_acceleration_structure(
    &self,
    positions: &[Vec3<f32>],
    indices: &[u32],
  ) -> Box<dyn GPUAccelerationStructureProvider>;
}
