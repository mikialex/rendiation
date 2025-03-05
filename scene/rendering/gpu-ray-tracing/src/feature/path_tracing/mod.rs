use crate::*;

mod bridge;
pub use bridge::*;

mod ray_gen;
use ray_gen::*;

mod frame_state;
use frame_state::*;

/// the main physical correct gpu ray tracing implementation
pub struct DeviceReferencePathTracingSystem {
  sbt: UpdateResultToken,
  executor: GPURaytracingPipelineExecutor,
  system: RtxSystemCore,
  shader_handles: PathTracingShaderHandles,
  state: Arc<RwLock<Option<PTRenderState>>>,
}

const MAX_RAY_DEPTH: u32 = 3;

impl DeviceReferencePathTracingSystem {
  pub fn new(rtx: &RtxSystemCore) -> Self {
    Self {
      sbt: Default::default(),
      executor: rtx.rtx_device.create_raytracing_pipeline_executor(),
      system: rtx.clone(),
      shader_handles: Default::default(),
      state: Default::default(),
    }
  }
  pub fn reset_sample(&self, gpu: &GPU) {
    if let Some(state) = self.state.write().as_mut() {
      state.reset(gpu);
    }
  }
}

impl RenderImplProvider<DeviceReferencePathTracingRenderer> for DeviceReferencePathTracingSystem {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, _: &GPU) {
    let sbt = GPUSbt::new(self.system.rtx_device.create_sbt(
      1,
      MAX_MODEL_COUNT_IN_SBT,
      GLOBAL_TLAS_MAX_RAY_STRIDE,
    ));
    let closest_hit = self.shader_handles.closest_hit;
    let sbt = MultiUpdateContainer::new(sbt).with_source(ReactiveQuerySbtUpdater {
      ray_ty_idx: PTRayType::Core as u32,
      source: global_watch()
        .watch_entity_set_untyped_key::<SceneModelEntity>()
        .collective_map(move |_| HitGroupShaderRecord {
          closest_hit: Some(closest_hit),
          any_hit: None,
          intersection: None,
        }),
    });

    self.sbt = source.register_multi_updater(sbt);
  }

  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.sbt);
  }

  fn create_impl(&self, res: &mut QueryResultCtx) -> DeviceReferencePathTracingRenderer {
    let sbt = res.take_multi_updater_updated::<GPUSbt>(self.sbt).unwrap();
    DeviceReferencePathTracingRenderer {
      shader_handles: self.shader_handles.clone(),
      max_ray_depth: MAX_RAY_DEPTH,
      sbt: sbt.target.clone(),
      executor: self.executor.clone(),
      frame_state: self.state.clone(),
    }
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
  executor: GPURaytracingPipelineExecutor,
  shader_handles: PathTracingShaderHandles,
  frame_state: Arc<RwLock<Option<PTRenderState>>>,
  max_ray_depth: u32,
  sbt: GPUSbt,
}

impl DeviceReferencePathTracingRenderer {
  pub fn render(
    &mut self,
    frame: &mut FrameCtx,
    base: &mut SceneRayTracingRendererBase,
    scene: EntityHandle<SceneEntity>,
    camera: EntityHandle<SceneCameraEntity>,
  ) -> GPU2DTextureView {
    let scene_tlas = base.scene_tlas.access(&scene).unwrap().clone();
    // bind tlas, see ShaderRayTraceCall::tlas_idx.
    base
      .rtx_system
      .create_acceleration_structure_system()
      .bind_tlas(&[scene_tlas.tlas_handle]);

    let render_size = clamp_size_by_area(frame.frame_size(), 512 * 512);
    let camera = base.camera.get_rtx_camera(camera);

    let mut rtx_encoder = base.rtx_system.create_raytracing_encoder();

    let trace_base_builder = base.rtx_system.create_tracer_base_builder();

    let mut state = self.frame_state.write();
    let state = state.deref_mut();
    if let Some(ao) = &state {
      if ao.radiance_buffer.size() != render_size {
        *state = None;
      }
    }

    let mut state = state
      .get_or_insert_with(|| PTRenderState::new(render_size, MAX_RAY_DEPTH, frame.gpu))
      .clone();
    let radiance_buffer = state
      .radiance_buffer
      .clone()
      .into_storage_texture_view_readwrite()
      .unwrap();

    let ray_gen = build_ray_gen_shader(
      &trace_base_builder,
      PTRayGenCtx {
        camera,
        radiance_buffer,
        config: state.config.clone(),
      },
      MAX_RAY_DEPTH as usize,
    );

    let closest = trace_base_builder
      .create_closest_hit_shader_base::<CorePathPayload>()
      .inject_ctx(PTRayClosestCtx {
        bindless_mesh: base.mesh.make_bindless_dispatcher(),
      })
      .map(|_, ctx| {
        let ao_cx = ctx.expect_custom_cx::<PTClosestCtxInvocation>();
        let closest_hit_ctx = ctx.expect_closest_hit_ctx();

        let normal = ao_cx.bindless_mesh.get_world_normal(closest_hit_ctx);
        let out_ray_origin = closest_hit_ctx.hit_world_position();

        // todo, impl material brdf model
        let out_ray_dir = normal.reflect(closest_hit_ctx.world_ray().direction);

        let payload = ctx.expect_payload::<CorePathPayload>();
        payload.next_ray_origin().store(out_ray_origin);
        payload.next_ray_dir().store(out_ray_dir);
        payload.normal().store(normal);
        payload.brdf().store(Vec3::splat(0.5));
        payload.pdf().store(1.);
        payload.missed().store(val(false).into_big_bool());
        //
      });

    let miss = trace_base_builder
      .create_miss_hit_shader_base::<CorePathPayload>()
      .map(|_, cx| {
        cx.payload::<CorePathPayload>().unwrap().store(
          ENode::<CorePathPayload> {
            sampled_radiance: val(Vec3::splat(1.)), // for testing return 10, use real env later
            next_ray_origin: zeroed_val(),
            next_ray_dir: zeroed_val(),
            pdf: zeroed_val(),
            missed: val(true).into_big_bool(),
            brdf: zeroed_val(),
            normal: zeroed_val(),
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

    source.set_execution_round_hint(self.max_ray_depth + 1);

    let sbt = self.sbt.inner.read();
    rtx_encoder.trace_ray(
      &source,
      &self.executor,
      dispatch_size(render_size),
      (*sbt).as_ref(),
    );

    state.next_sample(frame.gpu);
    state.radiance_buffer.clone()
  }
}

#[derive(Clone, Copy, ShaderStruct, Default)]
struct CorePathPayload {
  pub sampled_radiance: Vec3<f32>,
  pub brdf: Vec3<f32>,
  pub pdf: f32,
  pub normal: Vec3<f32>,
  pub next_ray_origin: Vec3<f32>,
  pub next_ray_dir: Vec3<f32>,
  pub missed: Bool,
}

#[std140_layout]
#[repr(C)]
#[derive(Clone, Copy, ShaderStruct)]
struct PTConfig {
  pub current_sample_count: u32,
  pub max_path_depth: u32,
}

impl PTConfig {
  pub fn new(max_path_depth: u32) -> Self {
    Self {
      max_path_depth,
      current_sample_count: 0,
      ..Zeroable::zeroed()
    }
  }
}

#[derive(Clone)]
struct PTRayClosestCtx {
  bindless_mesh: BindlessMeshDispatcher,
}

impl ShaderHashProvider for PTRayClosestCtx {
  shader_hash_type_id! {}
}

impl RayTracingCustomCtxProvider for PTRayClosestCtx {
  type Invocation = PTClosestCtxInvocation;

  fn build_invocation(&self, cx: &mut ShaderBindGroupBuilder) -> Self::Invocation {
    PTClosestCtxInvocation {
      bindless_mesh: self.bindless_mesh.build_bindless_mesh_rtx_access(cx),
    }
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    self.bindless_mesh.bind_bindless_mesh_rtx_access(builder);
  }
}

#[derive(Clone)]
struct PTClosestCtxInvocation {
  bindless_mesh: BindlessMeshRtxAccessInvocation,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
enum PTRayType {
  Core = 0,
}
impl PTRayType {
  fn to_sbt_cfg(self) -> RaySBTConfig {
    RaySBTConfig {
      offset: val(self as u32),
      stride: val(1),
    }
  }
}
