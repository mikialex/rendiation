use crate::*;

mod bridge;
pub use bridge::*;

mod ray_gen;
use ray_gen::*;

/// the main physical correct gpu ray tracing implementation
pub struct DeviceReferencePathTracingSystem {
  sbt: UpdateResultToken,
  executor: GPURaytracingPipelineExecutor,
  system: RtxSystemCore,
  shader_handles: PathTracingShaderHandles,
}

impl DeviceReferencePathTracingSystem {
  pub fn new(rtx: &RtxSystemCore) -> Self {
    Self {
      sbt: Default::default(),
      executor: rtx.rtx_device.create_raytracing_pipeline_executor(),
      system: rtx.clone(),
      shader_handles: Default::default(),
    }
  }
}

impl RenderImplProvider<DeviceReferencePathTracingRenderer> for DeviceReferencePathTracingSystem {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    todo!()
  }

  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    todo!()
  }

  fn create_impl(&self, res: &mut QueryResultCtx) -> DeviceReferencePathTracingRenderer {
    todo!()
  }
}

#[derive(Clone, PartialEq, Debug)]
struct PathTracingShaderHandles {
  ray_gen: ShaderHandle,
  closest_hit: ShaderHandle,
  secondary_closest: ShaderHandle,
  miss: ShaderHandle,
}
impl Default for PathTracingShaderHandles {
  fn default() -> Self {
    Self {
      ray_gen: ShaderHandle(0, RayTracingShaderStage::RayGeneration),
      closest_hit: ShaderHandle(0, RayTracingShaderStage::ClosestHit),
      secondary_closest: ShaderHandle(1, RayTracingShaderStage::ClosestHit),
      miss: ShaderHandle(0, RayTracingShaderStage::Miss),
    }
  }
}

pub struct DeviceReferencePathTracingRenderer {}

impl DeviceReferencePathTracingRenderer {
  pub fn render(
    &mut self,
    frame: &mut FrameCtx,
    base: &mut SceneRayTracingRendererBase,
    scene: EntityHandle<SceneEntity>,
    camera: EntityHandle<SceneCameraEntity>,
  ) -> GPU2DTextureView {
    let mut rtx_encoder = base.rtx_system.create_raytracing_encoder();

    todo!()
  }
}

struct CorePathPayload {
  pub sampled_radiance: Vec3<f32>,
  pub next_ray_origin: Vec3<f32>,
  pub next_ray_dir: Vec3<f32>,
}

#[std140_layout]
#[repr(C)]
#[derive(Clone, Copy, ShaderStruct)]
struct PTConfig {
  pub max_path_depth: u32,
  pub current_sample_count: u32,
}

#[derive(Clone)]
struct PTRayClosestCtx {
  bindless_mesh: BindlessMeshDispatcher,
  config: UniformBufferDataView<PTConfig>,
}

impl ShaderHashProvider for PTRayClosestCtx {
  shader_hash_type_id! {}
}

impl RayTracingCustomCtxProvider for PTRayClosestCtx {
  type Invocation = PTClosestCtxInvocation;

  fn build_invocation(&self, cx: &mut ShaderBindGroupBuilder) -> Self::Invocation {
    PTClosestCtxInvocation {
      bindless_mesh: self.bindless_mesh.build_bindless_mesh_rtx_access(cx),
      config: cx.bind_by(&self.config),
    }
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    self.bindless_mesh.bind_bindless_mesh_rtx_access(builder);
    builder.bind(&self.config);
  }
}

#[derive(Clone)]
struct PTClosestCtxInvocation {
  bindless_mesh: BindlessMeshRtxAccessInvocation,
  config: ShaderReadonlyPtrOf<PTConfig>,
}
