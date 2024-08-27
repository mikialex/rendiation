use crate::*;

pub trait GPURaytracingSystem {
  fn create_raytracing_device(&self) -> Box<dyn GPURayTracingDeviceProvider>;
  fn create_raytracing_encoder(&self) -> Box<dyn RayTracingPassEncoderProvider>;
  fn create_acceleration_structure_system(&self)
    -> Box<dyn GPUAccelerationStructureSystemProvider>;
}

pub trait RayTracingPassEncoderProvider {
  fn set_pipeline(&self, pipeline: &dyn GPURaytracingPipelineProvider);
  fn set_bindgroup(&self, index: u32, bindgroup: &rendiation_webgpu::BindGroup);
  fn trace_ray(&self, size: (u32, u32, u32), sbt: &dyn ShaderBindingTableProvider);
}

pub trait GPURaytracingPipelineProvider {
  fn access_impl(&mut self) -> &mut dyn Any;
}

pub trait GPURayTracingDeviceProvider {
  fn trace_op_base_builder(&self) -> RayCtxBaseBuilder;
  fn create_raytracing_pipeline(
    &self,
    desc: &GPURaytracingPipelineDescriptor,
  ) -> Box<dyn GPURaytracingPipelineProvider>;
  fn create_sbt(&self) -> Box<dyn ShaderBindingTableProvider>;
}

pub struct HitGroupShaderRecord {
  pub closet_hit: ShaderHandle,
  pub any_hit: Option<ShaderHandle>,
  pub intersection: Option<ShaderHandle>,
}

pub trait ShaderBindingTableProvider {
  fn resize(&mut self, mesh_count: u32, ray_type_count: u32);
  fn config_ray_generation(&mut self, s: ShaderHandle);
  fn config_hit_group(&mut self, mesh_idx: u32, hit_group: HitGroupShaderRecord);
  fn config_missing(&mut self, ray_ty_idx: u32, s: ShaderHandle);
  fn access_impl(&mut self) -> &mut dyn Any;
}

pub enum BottomLevelAccelerationStructureBuildSource {
  Triangles {
    positions: Vec<Vec3<f32>>,
    indices: Vec<u32>,
  },
  AABBs {
    aabbs: Vec<[f32; 6]>,
  },
}

pub trait GPUAccelerationStructureSystemProvider {
  fn create_top_level_acceleration_structure(
    &self,
    source: &[TopLevelAccelerationStructureSourceInstance],
  ) -> Box<dyn GPUAccelerationStructureInstanceProvider>;

  fn delete_top_level_acceleration_structure(
    &self,
    id: Box<dyn GPUAccelerationStructureInstanceProvider>,
  );

  fn create_bottom_level_acceleration_structure(
    &self,
    source: &[BottomLevelAccelerationStructureBuildSource],
  ) -> BottomLevelAccelerationStructureHandle;

  fn delete_bottom_level_acceleration_structure(&self, id: BottomLevelAccelerationStructureHandle);
}
pub struct BottomLevelAccelerationStructureHandle(pub u32);

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct TopLevelAccelerationStructureSourceInstance {
  pub transform: Mat4<f32>,
  pub instance_custom_index: u32,
  pub mask: u32,
  pub instance_shader_binding_table_record_offset: u32,
  pub flags: u32,
  pub acceleration_structure_handle: u64,
}

pub trait GPUAccelerationStructureInstanceProvider {
  fn access_impl(&self) -> &dyn Any;
}

pub trait IntersectionReporter {
  /// Invokes the current hit shader once an intersection shader has determined
  /// that a ray intersection has occurred. If the intersection occurred within
  /// the current ray interval, the any-hit shader corresponding to the current
  /// intersection shader is invoked. If the intersection is not ignored in the
  /// any-hit shader, <hitT> is committed as the new gl_RayTmaxEXT value of the
  /// current ray, <hitKind> is committed as the new value for gl_HitKindEXT, and
  /// true is returned. If either of those checks fails, then false is returned.
  /// If the value of <hitT> falls outside the current ray interval, the hit is
  /// rejected and false is returned.
  ///
  /// https://github.com/KhronosGroup/GLSL/blob/main/extensions/ext/GLSL_EXT_ray_tracing.txt#L954
  fn report_intersection(&self, hit_t: Node<f32>, hit_kind: Node<u32>) -> Node<bool>;
}