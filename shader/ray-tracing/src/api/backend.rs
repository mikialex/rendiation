use crate::*;

pub trait GPURaytracingSystem: DynClone {
  fn create_tracer_base_builder(&self) -> TraceFutureBaseBuilder;
  fn create_raytracing_device(&self) -> Box<dyn GPURayTracingDeviceProvider>;
  fn create_raytracing_encoder(&self) -> Box<dyn RayTracingEncoderProvider>;
  fn create_acceleration_structure_system(&self)
    -> Box<dyn GPUAccelerationStructureSystemProvider>;
}
clone_trait_object!(GPURaytracingSystem);

pub struct TraceFutureBaseBuilder {
  pub inner: Arc<dyn TraceFutureBaseProvider>,
}

impl TraceFutureBaseBuilder {
  pub fn create_ray_gen_shader_base(&self) -> Box<dyn TraceOperator<()>> {
    self.inner.create_ray_gen_shader_base()
  }

  pub fn create_closest_hit_shader_base<P: ShaderSizedValueNodeType>(
    &self,
  ) -> Box<dyn TraceOperator<()>> {
    self.inner.create_closest_hit_shader_base(P::sized_ty())
  }

  pub fn create_miss_hit_shader_base<P: ShaderSizedValueNodeType>(
    &self,
  ) -> Box<dyn TraceOperator<()>> {
    self.inner.create_miss_hit_shader_base(P::sized_ty())
  }
}

pub trait TraceFutureBaseProvider {
  fn create_ray_gen_shader_base(&self) -> Box<dyn TraceOperator<()>>;

  fn create_closest_hit_shader_base(
    &self,
    payload_ty: ShaderSizedValueType,
  ) -> Box<dyn TraceOperator<()>>;

  fn create_miss_hit_shader_base(
    &self,
    payload_ty: ShaderSizedValueType,
  ) -> Box<dyn TraceOperator<()>>;
}

pub trait RayTracingEncoderProvider {
  fn trace_ray(
    &mut self,
    pipeline: &GPURaytracingPipelineAndBindingSource,
    executor: &GPURaytracingPipelineExecutor,
    size: (u32, u32, u32),
    sbt: &dyn ShaderBindingTableProvider,
  );
}

/// an opaque rtx pipeline executor instance. cheap clonable.
pub trait GPURaytracingPipelineExecutorImpl: DynClone {
  fn access_impl(&self) -> &dyn Any;
}
dyn_clone::clone_trait_object!(GPURaytracingPipelineExecutorImpl);

#[derive(Clone)]
pub struct GPURaytracingPipelineExecutor {
  pub(crate) inner: Box<dyn GPURaytracingPipelineExecutorImpl>,
}

/// the ray tracing device abstraction.
pub trait GPURayTracingDeviceProvider: DynClone {
  /// create a pipeline executor. the executor is not the pipeline, the main reason for this api is that
  /// we want to cache the executor resource in user side and expose executor implementation in some cases.
  fn create_raytracing_pipeline_executor(&self) -> GPURaytracingPipelineExecutor;
  fn create_sbt(&self, mesh_count: u32, ray_type_count: u32)
    -> Box<dyn ShaderBindingTableProvider>;
}
dyn_clone::clone_trait_object!(GPURayTracingDeviceProvider);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HitGroupShaderRecord {
  pub closest_hit: Option<ShaderHandle>,
  pub any_hit: Option<ShaderHandle>,
  pub intersection: Option<ShaderHandle>,
}

pub trait ShaderBindingTableProvider {
  fn config_ray_generation(&mut self, s: ShaderHandle);
  fn config_hit_group(&mut self, tlas_idx: u32, ray_ty_idx: u32, hit_group: HitGroupShaderRecord);
  fn config_missing(&mut self, ray_ty_idx: u32, s: ShaderHandle);
  fn access_impl(&self) -> &dyn Any;
}

#[derive(Clone)]
pub struct BottomLevelAccelerationStructureBuildSource {
  pub geometry: BottomLevelAccelerationStructureBuildBuffer,
  pub flags: GeometryFlags,
}

#[derive(Clone)]
pub enum BottomLevelAccelerationStructureBuildBuffer {
  Triangles {
    positions: Vec<Vec3<f32>>,
    indices: Option<Vec<u32>>,
  },
  AABBs {
    aabbs: Vec<[f32; 6]>,
  },
}

pub trait GPUAccelerationStructureSystemProvider: DynClone + Send + Sync {
  fn create_comp_instance(&self) -> Box<dyn GPUAccelerationStructureSystemCompImplInstance>;
  fn create_top_level_acceleration_structure(
    &self,
    source: &[TopLevelAccelerationStructureSourceInstance],
  ) -> TlasHandle;

  fn delete_top_level_acceleration_structure(&self, id: TlasHandle);

  fn create_bottom_level_acceleration_structure(
    &self,
    source: &[BottomLevelAccelerationStructureBuildSource],
  ) -> BlasHandle;

  fn delete_bottom_level_acceleration_structure(&self, id: BlasHandle);
}
impl Clone for Box<dyn GPUAccelerationStructureSystemProvider> {
  fn clone(&self) -> Self {
    dyn_clone::clone_box(&**self)
  }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct TlasHandle(pub u32);

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct BlasHandle(pub u32);

/// https://learn.microsoft.com/en-us/windows/win32/api/d3d12/ns-d3d12-d3d12_raytracing_instance_desc
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct TopLevelAccelerationStructureSourceInstance {
  pub transform: Mat4<f32>,
  pub instance_custom_index: u32,
  pub mask: u32,
  pub instance_shader_binding_table_record_offset: u32,
  pub flags: GeometryInstanceFlags, // FLIP_FACING excludes whether transform is front/back
  pub acceleration_structure_handle: BlasHandle,
}

pub trait GPUAccelerationStructureInvocationInstance: DynClone {
  fn id(&self) -> Node<u32>;
}
clone_trait_object!(GPUAccelerationStructureInvocationInstance);

pub trait GPUAccelerationStructureInstanceProvider: DynClone + Send + Sync {
  fn create_invocation_instance(
    &self,
    builder: &mut ShaderBindGroupBuilder,
  ) -> Box<dyn GPUAccelerationStructureInvocationInstance>;
  fn bind_pass(&self, builder: &mut BindingBuilder);
  fn access_impl(&self) -> &dyn Any;
  fn id(&self) -> u32;
}
clone_trait_object!(GPUAccelerationStructureInstanceProvider);

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
