mod compute;
mod native;
// pub use compute::*;
// pub use native::*;

use crate::*;

pub trait GPUAccelerationStructureProvider {
  /// return optional closest hit
  fn traverse(
    &self,
    intersect: &dyn Fn(),
    any_hit: &dyn Fn(Node<WorldHitInfo>) -> Node<RayAnyHitBehavior>,
  ) -> DeviceOption<Node<WorldHitInfo>>;
}

pub trait GPURayTracingAccelerationStructureDeviceProvider {
  fn create_top_level_acceleration_structure(
    &self,
    source: &[TopLevelAccelerationStructureSourceInstance],
  ) -> Box<dyn GPUAccelerationStructureProvider>;

  fn create_bottom_level_acceleration_structure_by_triangles(
    &self,
    positions: &[Vec3<f32>],
    indices: &[u32],
  ) -> Box<dyn GPUAccelerationStructureProvider>;

  fn create_bottom_level_acceleration_structure_by_aabbs(
    &self,
    aabbs: &[[f32; 6]],
  ) -> Box<dyn GPUAccelerationStructureProvider>;
}

pub struct TopLevelAccelerationStructureSourceInstance {
  pub transform: Mat4<f32>,
  pub instance_custom_index: u32,
  pub mask: u32,
  pub instance_shader_binding_table_record_offset: u32,
  pub flags: u32,
  pub acceleration_structure_handle: u64,
}
