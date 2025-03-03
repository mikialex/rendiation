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
  miss: ShaderHandle,
}
impl Default for PathTracingShaderHandles {
  fn default() -> Self {
    Self {
      ray_gen: ShaderHandle(0, RayTracingShaderStage::RayGeneration),
      closest_hit: ShaderHandle(0, RayTracingShaderStage::ClosestHit),
      miss: ShaderHandle(0, RayTracingShaderStage::Miss),
    }
  }
}

pub struct DeviceReferencePathTracingRenderer {
  radiance_buffer: GPU2DTextureView,
  shader_handles: PathTracingShaderHandles,
}

impl DeviceReferencePathTracingRenderer {
  pub fn render(
    &mut self,
    frame: &mut FrameCtx,
    base: &mut SceneRayTracingRendererBase,
    scene: EntityHandle<SceneEntity>,
    camera: EntityHandle<SceneCameraEntity>,
  ) -> GPU2DTextureView {
    let camera = base.camera.get_rtx_camera(camera);

    let mut rtx_encoder = base.rtx_system.create_raytracing_encoder();

    let trace_base_builder = base.rtx_system.create_tracer_base_builder();

    let ray_gen = build_ray_gen_shader(
      &trace_base_builder,
      PTRayGenCtx {
        camera,
        radiance_buffer: todo!(),
        config: todo!(),
      },
    );

    let closest = trace_base_builder
      .create_closest_hit_shader_base::<CorePathPayload>()
      .inject_ctx(PTRayClosestCtx {
        bindless_mesh: todo!(),
        config: todo!(),
      })
      .map(|_, _| {
        //
      });

    let miss = trace_base_builder
      .create_miss_hit_shader_base::<CorePathPayload>()
      .map(|_, cx| {
        cx.payload::<CorePathPayload>().unwrap().store(
          ENode::<CorePathPayload> {
            sampled_radiance: val(Vec3::splat(10.)), // for testing return 10, use real env later
            next_ray_origin: zeroed_val(),
            next_ray_dir: zeroed_val(),
            missed: val(true).into_big_bool(),
          }
          .construct(),
        );
      });

    let mut source = GPURaytracingPipelineAndBindingSource::default();
    let handles = PathTracingShaderHandles {
      ray_gen: source.register_ray_gen(ray_gen),
      closest_hit: source.register_ray_closest_hit::<CorePathPayload>(closest, 1),
      miss: source.register_ray_miss::<CorePathPayload>(miss, 1),
    };
    assert_eq!(handles, self.shader_handles);

    source.set_execution_round_hint(todo!());

    // let sbt = self.sbt.inner.read();
    // rtx_encoder.trace_ray(
    //   &source,
    //   &self.executor,
    //   dispatch_size(render_size),
    //   (*sbt).as_ref(),
    // );

    // ao_state.next_sample(frame.gpu);
    // ao_state.ao_buffer.clone()
  }
}

#[derive(Clone, Copy, ShaderStruct, Default)]
struct CorePathPayload {
  pub sampled_radiance: Vec3<f32>,
  pub next_ray_origin: Vec3<f32>,
  pub next_ray_dir: Vec3<f32>,
  pub missed: Bool,
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
