use rendiation_texture_gpu_process::ToneMap;

use crate::*;

mod surface_bridge;
pub use surface_bridge::*;

mod lighting_bridge;
pub use lighting_bridge::*;

mod lighting_sampler;
pub use lighting_sampler::*;

mod lighting_source;
pub use lighting_source::*;

mod ray_gen;
use ray_gen::*;

mod ray_hit;
use ray_hit::*;

mod ray_miss;
use ray_miss::*;

mod frame_state;
use frame_state::*;

pub fn use_rtx_pt_renderer(
  cx: &mut impl QueryGPUHookCx,
  rtx: &RtxSystemCore,
) -> Option<DeviceReferencePathTracingRenderer> {
  todo!()
}

/// the main physical correct gpu ray tracing implementation
pub struct DeviceReferencePathTracingSystem {
  sbt: QueryToken,
  executor: GPURaytracingPipelineExecutor,
  system: RtxSystemCore,
  shader_handles: PathTracingShaderHandles,
  state: Arc<RwLock<Option<PTRenderState>>>,
  source_set: QueryCtxSetInfo,
  gpu: GPU,
}

const MAX_RAY_DEPTH: u32 = 3;

impl DeviceReferencePathTracingSystem {
  pub fn new(rtx: &RtxSystemCore, gpu: &GPU) -> Self {
    Self {
      sbt: Default::default(),
      executor: rtx.rtx_device.create_raytracing_pipeline_executor(),
      system: rtx.clone(),
      shader_handles: Default::default(),
      state: Default::default(),
      source_set: Default::default(),
      gpu: gpu.clone(),
    }
  }
  pub fn reset_sample(&self) {
    if let Some(state) = self.state.write().as_mut() {
      state.reset(&self.gpu);
    }
  }
}

impl QueryBasedFeature<DeviceReferencePathTracingRenderer> for DeviceReferencePathTracingSystem {
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, _: &GPU) {
    qcx.record_new_registered(&mut self.source_set);
    let handles = PathTracingShaderHandles::default();
    let mut sbt =
      self
        .system
        .rtx_device
        .create_sbt(1, MAX_MODEL_COUNT_IN_SBT, GLOBAL_TLAS_MAX_RAY_STRIDE);

    sbt.config_ray_generation(handles.ray_gen);
    sbt.config_missing(PTRayType::Core as u32, handles.miss);
    sbt.config_missing(PTRayType::ShadowTest as u32, handles.shadow_test_miss);
    let sbt = GPUSbt::new(sbt);
    let core_closest_hit = self.shader_handles.closest_hit;
    let shadow_closest_hit = self.shader_handles.shadow_test_hit;
    let sbt = MultiUpdateContainer::new(sbt)
      .with_source(ReactiveQuerySbtUpdater {
        ray_ty_idx: PTRayType::Core as u32,
        source: global_watch()
          .watch_entity_set_untyped_key::<SceneModelEntity>()
          .collective_map(move |_| HitGroupShaderRecord {
            closest_hit: Some(core_closest_hit),
            any_hit: None,
            intersection: None,
          }),
      })
      .with_source(ReactiveQuerySbtUpdater {
        ray_ty_idx: PTRayType::ShadowTest as u32,
        source: global_watch()
          .watch_entity_set_untyped_key::<SceneModelEntity>()
          .collective_map(move |_| HitGroupShaderRecord {
            closest_hit: Some(shadow_closest_hit),
            any_hit: None,
            intersection: None,
          }),
      });

    self.sbt = qcx.register_multi_updater(sbt);
    qcx.end_record(&mut self.source_set);
  }

  fn deregister(&mut self, qcx: &mut ReactiveQueryCtx) {
    qcx.deregister(&mut self.sbt);
  }

  fn create_impl(&self, cx: &mut QueryResultCtx) -> DeviceReferencePathTracingRenderer {
    let sbt = cx.take_multi_updater_updated::<GPUSbt>(self.sbt).unwrap();

    let r = DeviceReferencePathTracingRenderer {
      shader_handles: self.shader_handles.clone(),
      max_ray_depth: MAX_RAY_DEPTH,
      sbt: sbt.target.clone(),
      executor: self.executor.clone(),
      frame_state: self.state.clone(),
      gpu: self.gpu.clone(),
    };

    if cx.has_any_changed_in_set(&self.source_set) {
      r.reset_sample();
    }
    r
  }
}

#[derive(Clone, PartialEq, Debug)]
struct PathTracingShaderHandles {
  ray_gen: ShaderHandle,
  closest_hit: ShaderHandle,
  shadow_test_hit: ShaderHandle,
  miss: ShaderHandle,
  shadow_test_miss: ShaderHandle,
}
impl Default for PathTracingShaderHandles {
  fn default() -> Self {
    Self {
      ray_gen: ShaderHandle(0, RayTracingShaderStage::RayGeneration),
      closest_hit: ShaderHandle(0, RayTracingShaderStage::ClosestHit),
      shadow_test_hit: ShaderHandle(1, RayTracingShaderStage::ClosestHit),
      miss: ShaderHandle(0, RayTracingShaderStage::Miss),
      shadow_test_miss: ShaderHandle(1, RayTracingShaderStage::Miss),
    }
  }
}

pub struct DeviceReferencePathTracingRenderer {
  executor: GPURaytracingPipelineExecutor,
  shader_handles: PathTracingShaderHandles,
  frame_state: Arc<RwLock<Option<PTRenderState>>>,
  max_ray_depth: u32,
  sbt: GPUSbt,
  gpu: GPU,
}

impl DeviceReferencePathTracingRenderer {
  pub fn reset_sample(&self) {
    if let Some(state) = self.frame_state.write().as_mut() {
      state.reset(&self.gpu);
    }
  }

  pub fn render(
    &mut self,
    frame: &mut FrameCtx,
    rtx_system: &dyn GPURaytracingSystem,
    base: &mut SceneRayTracingRendererBase,
    scene: EntityHandle<SceneEntity>,
    camera: EntityHandle<SceneCameraEntity>,
    tonemap: &ToneMap,
    background: &SceneBackgroundRenderer,
  ) -> GPU2DTextureView {
    let scene_tlas = base.scene_tlas.access(&scene).unwrap().clone();
    // bind tlas, see ShaderRayTraceCall::tlas_idx.
    rtx_system
      .create_acceleration_structure_system()
      .bind_tlas(&[scene_tlas.tlas_handle]);

    let render_size = clamp_size_by_area(frame.frame_size(), 512 * 512);
    let camera = base.camera.get_rtx_camera(camera);

    let mut rtx_encoder = rtx_system.create_raytracing_encoder();

    let trace_base_builder = rtx_system.create_tracer_base_builder();

    let mut state = self.frame_state.write();
    let state = state.deref_mut();
    if let Some(s) = &state {
      if s.radiance_buffer.size() != render_size {
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
        result_buffer: radiance_buffer,
        config: state.config.clone(),
        tonemap: tonemap.clone(),
      },
      MAX_RAY_DEPTH as usize,
    );

    let lighting = ScenePTLighting {
      scene_data: base.lighting.clone(),
      scene_id: base.scene_ids.get(&scene).unwrap().clone(),
    };

    let closest = build_ray_hit_shader(
      &trace_base_builder,
      PTRayClosestCtx {
        bindless_mesh: base.mesh.make_bindless_dispatcher(),
        surface: Box::new(base.material.clone()),
        config: state.config.clone(),
        lighting: Box::new(lighting),
      },
    );

    let miss_ctx = PTRayMissCtx::new(background, scene, frame.gpu);
    let miss = build_ray_miss_shader(&trace_base_builder, miss_ctx);

    let shadow_test_closest = trace_base_builder
      .create_closest_hit_shader_base::<ShaderTestPayload>()
      .map(|_, cx| {
        cx.expect_payload::<ShaderTestPayload>()
          .radiance()
          .store(Vec3::zero());
      });

    let shadow_miss = trace_base_builder
      .create_miss_hit_shader_base::<ShaderTestPayload>()
      .map(|_, _| {
        // do nothing
      });

    let mut source = GPURaytracingPipelineAndBindingSource::default();
    let handles = PathTracingShaderHandles {
      ray_gen: source.register_ray_gen(ray_gen),
      closest_hit: source.register_ray_closest_hit::<CorePathPayload>(closest, 1),
      shadow_test_hit: source.register_ray_closest_hit::<ShaderTestPayload>(shadow_test_closest, 1),
      miss: source.register_ray_miss::<CorePathPayload>(miss, 1),
      shadow_test_miss: source.register_ray_miss::<ShaderTestPayload>(shadow_miss, 1),
    };
    assert_eq!(handles, self.shader_handles);

    source.set_execution_round_hint(self.max_ray_depth * 5);
    // this is 2 because when previous ray is reading back, their is no empty space for allocate new ray
    source.max_in_flight_trace_ray = 2;

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
  pub surface_radiance: Vec3<f32>,
  pub brdf: Vec3<f32>,
  pub pdf: f32,
  pub normal: Vec3<f32>,
  pub next_ray_origin: Vec3<f32>,
  pub next_ray_dir: Vec3<f32>,
  pub missed: Bool,
}

#[derive(Clone, Copy, ShaderStruct, Default)]
struct ShaderTestPayload {
  pub radiance: Vec3<f32>,
  pub light_sample_dir: Vec3<f32>,
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

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
enum PTRayType {
  Core = 0,
  ShadowTest = 1,
}
impl PTRayType {
  fn to_sbt_cfg(self) -> RaySBTConfig {
    RaySBTConfig {
      offset: val(self as u32),
      stride: val(2),
    }
  }
}
