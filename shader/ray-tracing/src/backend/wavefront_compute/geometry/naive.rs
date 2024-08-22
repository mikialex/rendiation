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

struct NaiveSahBVHSystem {
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

impl GPUAccelerationStructureCompImplInstance for NaiveSahBVHSystem {
  fn build_shader(
    &self,
    compute_cx: &mut ShaderComputePipelineBuilder,
  ) -> Box<dyn GPUAccelerationStructureCompImplInvocationTraversable> {
    todo!()
  }

  fn bind_pass(&self, builder: &mut BindingBuilder) {
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

impl GPUAccelerationStructureCompImplInvocationTraversable for NaiveSahBVHInvocationInstance {
  fn traverse(
    &self,
    trace_payload: ENode<ShaderRayTraceCallStoragePayload>,
    intersect: &dyn Fn(&RayIntersectCtx, &dyn IntersectionReporter),
    any_hit: &dyn Fn(&RayAnyHitCtx) -> Node<RayAnyHitBehavior>,
  ) -> DeviceOption<RayClosestHitCtx> {
    loop_by(|_| {
      loop_by(|_| {
        //
      })
    });
    todo!()
  }
}

impl GPUAccelerationStructureInstanceBuilder for NaiveSahBVHSystem {
  fn create_top_level_acceleration_structure(
    &self,
    source: &[TopLevelAccelerationStructureSourceInstance],
  ) -> Box<dyn GPUAccelerationStructureInstanceProvider> {
    todo!()
  }

  fn delete_top_level_acceleration_structure(
    &self,
    id: Box<dyn GPUAccelerationStructureInstanceProvider>,
  ) {
    todo!()
  }

  fn create_bottom_level_acceleration_structure_by_triangles(
    &self,
    positions: &[Vec3<f32>],
    indices: &[u32],
  ) -> BottomLevelAccelerationStructureHandle {
    todo!()
  }

  fn create_bottom_level_acceleration_structure_by_aabbs(
    &self,
    aabbs: &[[f32; 6]],
  ) -> BottomLevelAccelerationStructureHandle {
    todo!()
  }

  fn delete_bottom_level_acceleration_structure(&self, id: BottomLevelAccelerationStructureHandle) {
    todo!()
  }
}
