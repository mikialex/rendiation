mod compute;
mod native;
pub use compute::*;

// pub use native::*;
use crate::*;

pub trait GPUAccelerationStructureInvocationTraversable {
  /// return optional closest hit
  fn traverse(
    &self,
    intersect: &dyn Fn(),
    any_hit: &dyn Fn(Node<WorldHitInfo>) -> Node<RayAnyHitBehavior>,
  ) -> DeviceOption<Node<WorldHitInfo>>;
}

pub trait GPUAccelerationStructureInstance {
  fn build_shader(
    &self,
    compute_cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn GPUAccelerationStructureInvocationTraversable>;
  fn bind_pass(&self, pass: &mut GPUComputePass);

  fn handle(&self) -> u32;
}

pub trait GPUAccelerationStructureInstanceBuilder {
  fn create_top_level_acceleration_structure(
    &self,
    source: &[TopLevelAccelerationStructureSourceInstance],
  ) -> Box<dyn GPUAccelerationStructureInstance>;

  fn delete_top_level_acceleration_structure(&self, id: Box<dyn GPUAccelerationStructureInstance>);

  fn create_bottom_level_acceleration_structure_by_triangles(
    &self,
    positions: &[Vec3<f32>],
    indices: &[u32],
  ) -> u32;

  fn create_bottom_level_acceleration_structure_by_aabbs(&self, aabbs: &[[f32; 6]]) -> u32;

  fn delete_bottom_level_acceleration_structure(&self, id: u32);
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct TopLevelAccelerationStructureSourceInstance {
  pub transform: Mat4<f32>,
  pub instance_custom_index: u32,
  pub mask: u32,
  pub instance_shader_binding_table_record_offset: u32,
  pub flags: u32,
  pub acceleration_structure_handle: u64,
}
