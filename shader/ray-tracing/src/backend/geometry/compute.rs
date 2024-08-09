use rendiation_space_algorithm::bvh::*;

use crate::*;

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, PartialEq, Debug, ShaderStruct)]
pub struct TopLevelAccelerationStructureSourceDeviceInstance {
  pub transform: Mat4<f32>,
  pub instance_custom_index: u32,
  pub mask: u32,
  pub instance_shader_binding_table_record_offset: u32,
  pub flags: u32,
  pub acceleration_structure_handle: u32,
}

#[repr(C)]
#[std430_layout]
#[derive(Clone, Copy, PartialEq, Debug, ShaderStruct)]
struct DeviceBVHNode {
  pub aabb_min: Vec3<f32>,
  pub aabb_max: Vec3<f32>,
}

struct NaiveSahBVHHostBuilder {
  tlas_meta_info: StorageBufferReadOnlyDataView<[u32]>,
  tlas_bvh_forest: StorageBufferReadOnlyDataView<[DeviceBVHNode]>,
  tlas_data_indices: StorageBufferReadOnlyDataView<[u32]>,
  tlas_data: StorageBufferReadOnlyDataView<[TopLevelAccelerationStructureSourceDeviceInstance]>,

  blas_meta_info: StorageBufferReadOnlyDataView<[u32]>,
  blas_bvh_forest: StorageBufferReadOnlyDataView<[DeviceBVHNode]>,

  indices: StorageBufferReadOnlyDataView<[u32]>,
  triangles: StorageBufferReadOnlyDataView<[Vec3<f32>]>,
  boxes: StorageBufferReadOnlyDataView<[Vec3<f32>]>,
}

struct GPUNaiveSahBVHInstance {
  instance: UniformBufferDataView<u32>,
  handle: u32,
}

impl GPUAccelerationStructureInstance for GPUNaiveSahBVHInstance {
  fn handle(&self) -> u32 {
    self.handle
  }
  fn build_shader(
    &self,
    compute_cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn GPUAccelerationStructureInvocationTraversable> {
    todo!()
  }

  fn bind_pass(&self, pass: &mut GPUComputePass) {
    todo!()
  }
}

pub struct NaiveSahBVHInvocationInstance {
  instance: UniformNode<u32>,
  tlas_meta_info: StorageNode<[u32]>,
  tlas_bvh_forest: StorageNode<[u32]>,
  tlas_data_indices: StorageNode<[u32]>,
  tlas_data: StorageNode<[u32]>,
  blas_meta_info: StorageNode<[u32]>,
  blas_bvh_forest: StorageNode<[u32]>,
  indices: StorageNode<[u32]>,
  triangles: StorageNode<[u32]>,
  boxes: StorageNode<[u32]>,
}

impl GPUAccelerationStructureInvocationTraversable for NaiveSahBVHInvocationInstance {
  fn traverse(
    &self,
    intersect: &dyn Fn(),
    any_hit: &dyn Fn(Node<WorldHitInfo>) -> Node<RayAnyHitBehavior>,
  ) -> DeviceOption<Node<WorldHitInfo>> {
    loop_by(|_| {
      loop_by(|_| {
        //
      })
    });
    todo!()
  }
}

impl GPUAccelerationStructureInstanceBuilder for NaiveSahBVHHostBuilder {
  fn create_top_level_acceleration_structure(
    &self,
    source: &[TopLevelAccelerationStructureSourceInstance],
  ) -> Box<dyn GPUAccelerationStructureInstance> {
    todo!()
  }

  fn delete_top_level_acceleration_structure(&self, id: Box<dyn GPUAccelerationStructureInstance>) {
    todo!()
  }

  fn create_bottom_level_acceleration_structure_by_triangles(
    &self,
    positions: &[Vec3<f32>],
    indices: &[u32],
  ) -> u32 {
    todo!()
  }

  fn delete_bottom_level_acceleration_structure(&self, id: u32) {
    todo!()
  }

  fn create_bottom_level_acceleration_structure_by_aabbs(&self, aabbs: &[[f32; 6]]) -> u32 {
    todo!()
  }
}
