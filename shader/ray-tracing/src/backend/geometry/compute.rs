use rendiation_space_algorithm::bvh::*;

use crate::*;

struct NaiveSahBVHHostBuilder {
  meta_info: StorageBufferReadOnlyDataView<[u32]>,
  bvh_forest: StorageBufferReadOnlyDataView<[u32]>,
  indices: StorageBufferReadOnlyDataView<[u32]>,
  triangles: StorageBufferReadOnlyDataView<[u32]>,
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
