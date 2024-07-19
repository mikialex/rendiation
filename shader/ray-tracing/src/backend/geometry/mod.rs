mod compute;
pub use compute::*;
mod native;
pub use native::*;

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
  fn create_acceleration_structure(
    &self,
    positions: &[Vec3<f32>],
    indices: &[u32],
  ) -> Box<dyn GPUAccelerationStructureProvider>;
}
