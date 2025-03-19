use rendiation_device_ray_tracing::RayFlagConfigRaw::RAY_FLAG_ACCEPT_FIRST_HIT_AND_END_SEARCH;
use rendiation_shader_library::sampling::{hammersley_2d_fn, sample_hemisphere_cos_fn, tbn_fn};
use rendiation_texture_core::Size;

use crate::*;

const MAX_SAMPLE: u32 = 256;

pub struct RayTracingAORenderSystem {
  sbt: QueryToken,
  executor: GPURaytracingPipelineExecutor,
  system: RtxSystemCore,
  shader_handles: AOShaderHandles,
  ao_state: Arc<RwLock<Option<AORenderState>>>,
  gpu: GPU,
  source_set: QueryCtxSetInfo,
}

impl RayTracingAORenderSystem {
  pub fn new(rtx: &RtxSystemCore, gpu: &GPU) -> Self {
    Self {
      sbt: Default::default(),
      executor: rtx.rtx_device.create_raytracing_pipeline_executor(),
      system: rtx.clone(),
      ao_state: Default::default(),
      shader_handles: Default::default(),
      source_set: Default::default(),
      gpu: gpu.clone(),
    }
  }
  pub fn reset_ao_sample(&self) {
    if let Some(state) = self.ao_state.write().as_mut() {
      state.reset(&self.gpu);
    }
  }
}

#[derive(Clone)]
struct AORenderState {
  ao_buffer: GPU2DTextureView,
  sample_count_host: Arc<RwLock<u32>>,
  sample_count: UniformBufferDataView<Vec4<u32>>,
}

impl AORenderState {
  fn new(size: Size, gpu: &GPU) -> Self {
    Self {
      ao_buffer: create_empty_2d_texture_view(
        gpu,
        size,
        basic_texture_usages(),
        TextureFormat::Rgba8Unorm,
      ),
      sample_count_host: Default::default(),
      sample_count: create_uniform(Vec4::zeroed(), gpu),
    }
  }
  fn next_sample(&mut self, gpu: &GPU) {
    let current = *self.sample_count_host.read();
    self.sample_count.write_at(&gpu.queue, &(current + 1), 0);
    *self.sample_count_host.write() = current + 1;
  }
  fn reset(&mut self, gpu: &GPU) {
    *self.sample_count_host.write() = 0;
    // buffer should be reset automatically in rtx pipeline
    self.sample_count.write_at(&gpu.queue, &0_u32, 0);
  }
}

#[derive(Clone, PartialEq, Debug)]
struct AOShaderHandles {
  ray_gen: ShaderHandle,
  closest_hit: ShaderHandle,
  secondary_closest: ShaderHandle,
  miss: ShaderHandle,
}

impl Default for AOShaderHandles {
  fn default() -> Self {
    Self {
      ray_gen: ShaderHandle(0, RayTracingShaderStage::RayGeneration),
      closest_hit: ShaderHandle(0, RayTracingShaderStage::ClosestHit),
      secondary_closest: ShaderHandle(1, RayTracingShaderStage::ClosestHit),
      miss: ShaderHandle(0, RayTracingShaderStage::Miss),
    }
  }
}

impl QueryBasedFeature<SceneRayTracingAORenderer> for RayTracingAORenderSystem {
  type Context = GPU;
  fn register(&mut self, qcx: &mut ReactiveQueryCtx, _: &GPU) {
    qcx.record_new_registered(&mut self.source_set);
    let handles = AOShaderHandles::default();
    let mut sbt =
      self
        .system
        .rtx_device
        .create_sbt(1, MAX_MODEL_COUNT_IN_SBT, GLOBAL_TLAS_MAX_RAY_STRIDE);

    sbt.config_ray_generation(handles.ray_gen);
    sbt.config_missing(AORayType::Primary as u32, handles.miss);
    sbt.config_missing(AORayType::AOTest as u32, handles.miss);

    let sbt = GPUSbt::new(sbt);
    let closest_hit = self.shader_handles.closest_hit;
    let secondary_closest = self.shader_handles.secondary_closest;
    let sbt = MultiUpdateContainer::new(sbt)
      .with_source(ReactiveQuerySbtUpdater {
        ray_ty_idx: AORayType::Primary as u32,
        source: global_watch()
          .watch_entity_set_untyped_key::<SceneModelEntity>()
          .collective_map(move |_| HitGroupShaderRecord {
            closest_hit: Some(closest_hit),
            any_hit: None,
            intersection: None,
          }),
      })
      .with_source(ReactiveQuerySbtUpdater {
        ray_ty_idx: AORayType::AOTest as u32,
        source: global_watch()
          .watch_entity_set_untyped_key::<SceneModelEntity>()
          .collective_map(move |_| HitGroupShaderRecord {
            closest_hit: Some(secondary_closest),
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

  fn create_impl(&self, cx: &mut QueryResultCtx) -> SceneRayTracingAORenderer {
    let sbt = cx.take_multi_updater_updated::<GPUSbt>(self.sbt).unwrap();
    let r = SceneRayTracingAORenderer {
      executor: self.executor.clone(),
      sbt: sbt.target.clone(),
      ao_state: self.ao_state.clone(),
      shader_handles: self.shader_handles.clone(),
      gpu: self.gpu.clone(),
    };

    if cx.has_any_changed_in_set(&self.source_set) {
      r.reset_sample();
    }
    r
  }
}

pub struct SceneRayTracingAORenderer {
  executor: GPURaytracingPipelineExecutor,
  sbt: GPUSbt,
  ao_state: Arc<RwLock<Option<AORenderState>>>,
  shader_handles: AOShaderHandles,
  gpu: GPU,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
enum AORayType {
  Primary = 0,
  AOTest = 1,
}

impl AORayType {
  fn to_sbt_cfg(self) -> RaySBTConfig {
    RaySBTConfig {
      offset: val(self as u32),
      stride: val(2),
    }
  }
}

impl SceneRayTracingAORenderer {
  pub fn reset_sample(&self) {
    if let Some(state) = self.ao_state.write().as_mut() {
      state.reset(&self.gpu);
    }
  }

  #[instrument(name = "SceneRayTracingAORenderer rendering", skip_all)]
  pub fn render(
    &mut self,
    frame: &mut FrameCtx,
    base: &mut SceneRayTracingRendererBase,
    scene: EntityHandle<SceneEntity>,
    camera: EntityHandle<SceneCameraEntity>,
  ) -> GPU2DTextureView {
    let scene_tlas = base.scene_tlas.access(&scene).unwrap().clone();
    let render_size = clamp_size_by_area(frame.frame_size(), 512 * 512);

    let mut ao_state = self.ao_state.write();
    let ao_state = ao_state.deref_mut();
    if let Some(ao) = &ao_state {
      if ao.ao_buffer.size() != render_size {
        *ao_state = None;
      }
    }

    let mut ao_state = ao_state
      .get_or_insert_with(|| AORenderState::new(render_size, frame.gpu))
      .clone();
    let ao_buffer_rw = ao_state
      .ao_buffer
      .clone()
      .into_storage_texture_view_readwrite()
      .unwrap();

    let mut source = GPURaytracingPipelineAndBindingSource::default();

    let camera = base.camera.get_rtx_camera(camera);

    let trace_base_builder = base.rtx_system.create_tracer_base_builder();

    // bind tlas, see ShaderRayTraceCall::tlas_idx.
    base
      .rtx_system
      .create_acceleration_structure_system()
      .bind_tlas(&[scene_tlas.tlas_handle]);

    let mut rtx_encoder = base.rtx_system.create_raytracing_encoder();

    let ray_gen_shader = trace_base_builder
      .create_ray_gen_shader_base()
      .inject_ctx(RayTracingAORayGenCtx {
        camera,
        ao_buffer: ao_buffer_rw,
        ao_sample_count: ao_state.sample_count.clone(),
      })
      .then_trace(|_, ctx| {
        let rg_cx = ctx.expect_ray_gen_ctx();
        let ao_cx = ctx.expect_custom_cx::<RayTracingAORayGenCtxInvocation>();
        let sampler =
          &PCGRandomSampler::from_ray_ctx_and_sample_index(rg_cx, ao_cx.ao_sample_count.load().x());
        let ray =
          ao_cx
            .camera
            .generate_ray(rg_cx.launch_id().xy(), rg_cx.launch_size().xy(), sampler);

        let trace_call = ShaderRayTraceCall {
          tlas_idx: val(0), // only one tlas, select first
          ray_flags: val(RayFlagConfigRaw::RAY_FLAG_CULL_BACK_FACING_TRIANGLES as u32),
          cull_mask: val(u32::MAX),
          sbt_ray_config: AORayType::Primary.to_sbt_cfg(),
          miss_index: val(0),
          ray,
          range: ShaderRayRange::default(),
        };

        (val(true), trace_call, val(0.)) // zero means occluded, use miss shader to write one
      })
      .map(|(_, payload), ctx| {
        let ao_cx = ctx.expect_custom_cx::<RayTracingAORayGenCtxInvocation>();
        let position = ctx.expect_ray_gen_ctx().launch_id().xy();

        let ao_sample_count = ao_cx.ao_sample_count.load().x();
        let previous_sample_count = ao_sample_count.into_f32();
        let all_sample_count = previous_sample_count + val(1.0);

        let previous_sample_acc = ao_cx.ao_buffer.load_storage_texture_texel(position).x();
        let new_sample_acc =
          (previous_sample_acc * previous_sample_count + payload) / all_sample_count;
        let new_sample_acc = new_sample_acc.splat::<Vec3<_>>();

        if_by(ao_sample_count.less_than(val(MAX_SAMPLE)), || {
          ao_cx
            .ao_buffer
            .write_texel(position, (new_sample_acc, val(1.0)).into());
        })
        .else_by(|| {
          ao_cx.ao_buffer.write_texel(
            position,
            (previous_sample_acc.splat::<Vec3<_>>(), val(1.0)).into(),
          );
        });
      });

    type RayGenTracePayload = f32; // occlusion
    let bindless_mesh = base.mesh.make_bindless_dispatcher();
    let ao_closest = trace_base_builder
      .create_closest_hit_shader_base::<RayGenTracePayload>()
      .inject_ctx(RayTracingAORayClosestCtx {
        bindless_mesh,
        ao_sample_count: ao_state.sample_count.clone(),
      })
      .then_trace(|_, ctx| {
        let ao_cx = ctx.expect_custom_cx::<RayTracingAORayClosestCtxInvocation>();
        let closest_hit_ctx = ctx.expect_closest_hit_ctx();

        let (shading_normal, _) = ao_cx.bindless_mesh.get_world_normal(closest_hit_ctx);
        let hit_normal_tbn = tbn_fn(shading_normal);

        let origin = closest_hit_ctx.hit_world_position();

        let random = hammersley_2d_fn(ao_cx.ao_sample_count.load().x(), val(MAX_SAMPLE));
        let direction = hit_normal_tbn * sample_hemisphere_cos_fn(random);

        let ray = ShaderRay { origin, direction };

        let trace_call = ShaderRayTraceCall {
          tlas_idx: val(0),
          ray_flags: val(RAY_FLAG_ACCEPT_FIRST_HIT_AND_END_SEARCH as u32),
          cull_mask: val(u32::MAX),
          sbt_ray_config: AORayType::AOTest.to_sbt_cfg(),
          miss_index: val(0), // using the sample miss shader as primary ray
          ray,
          range: ShaderRayRange {
            min: val(0.01),
            max: val(100.0),
          },
        };

        (val(true), trace_call, val(1.))
      })
      .map(|(_, payload), ctx| ctx.expect_payload::<f32>().store(payload));

    source.max_in_flight_trace_ray(2);
    let handles = AOShaderHandles {
      ray_gen: source.register_ray_gen(ray_gen_shader),
      closest_hit: source.register_ray_closest_hit::<RayGenTracePayload>(ao_closest, 1),
      secondary_closest: source.register_ray_closest_hit::<RayGenTracePayload>(
        trace_base_builder
          .create_closest_hit_shader_base::<RayGenTracePayload>()
          .map(|_, ctx| ctx.payload::<f32>().unwrap().store(val(0.))),
        1,
      ),
      miss: source.register_ray_miss::<RayGenTracePayload>(
        trace_base_builder
          .create_miss_hit_shader_base::<RayGenTracePayload>()
          .map(|_, cx| {
            cx.payload::<RayGenTracePayload>().unwrap().store(val(1.0));
          }),
        1,
      ),
    };
    assert_eq!(handles, self.shader_handles);
    source.set_execution_round_hint(8);

    let sbt = self.sbt.inner.read();
    rtx_encoder.trace_ray(
      &source,
      &self.executor,
      dispatch_size(render_size),
      (*sbt).as_ref(),
    );

    ao_state.next_sample(frame.gpu);
    ao_state.ao_buffer.clone()
  }
}

#[derive(Clone)]
struct RayTracingAORayGenCtx {
  camera: Box<dyn RtxCameraRenderComponent>,
  ao_buffer: StorageTextureViewReadWrite<GPU2DTextureView>,
  ao_sample_count: UniformBufferDataView<Vec4<u32>>,
}

impl ShaderHashProvider for RayTracingAORayGenCtx {
  shader_hash_type_id! {}
  fn hash_pipeline(&self, hasher: &mut PipelineHasher) {
    self.camera.hash_pipeline(hasher);
  }
}

impl RayTracingCustomCtxProvider for RayTracingAORayGenCtx {
  type Invocation = RayTracingAORayGenCtxInvocation;

  fn build_invocation(&self, cx: &mut ShaderBindGroupBuilder) -> Self::Invocation {
    RayTracingAORayGenCtxInvocation {
      camera: self.camera.build_invocation(cx),
      ao_buffer: cx.bind_by(&self.ao_buffer),
      ao_sample_count: cx.bind_by(&self.ao_sample_count),
    }
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    self.camera.bind(builder);
    builder.bind(&self.ao_buffer);
    builder.bind(&self.ao_sample_count);
  }
}

#[derive(Clone)]
struct RayTracingAORayGenCtxInvocation {
  camera: Box<dyn RtxCameraRenderInvocation>,
  ao_buffer: BindingNode<ShaderStorageTextureRW2D>,
  ao_sample_count: ShaderReadonlyPtrOf<Vec4<u32>>,
}

#[derive(Clone)]
struct RayTracingAORayClosestCtx {
  bindless_mesh: BindlessMeshDispatcher,
  ao_sample_count: UniformBufferDataView<Vec4<u32>>,
}

impl ShaderHashProvider for RayTracingAORayClosestCtx {
  shader_hash_type_id! {}
}

impl RayTracingCustomCtxProvider for RayTracingAORayClosestCtx {
  type Invocation = RayTracingAORayClosestCtxInvocation;

  fn build_invocation(&self, cx: &mut ShaderBindGroupBuilder) -> Self::Invocation {
    RayTracingAORayClosestCtxInvocation {
      bindless_mesh: self.bindless_mesh.build_bindless_mesh_rtx_access(cx),
      ao_sample_count: cx.bind_by(&self.ao_sample_count),
    }
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    self.bindless_mesh.bind_bindless_mesh_rtx_access(builder);
    builder.bind(&self.ao_sample_count);
  }
}

#[derive(Clone)]
struct RayTracingAORayClosestCtxInvocation {
  bindless_mesh: BindlessMeshRtxAccessInvocation,
  ao_sample_count: ShaderReadonlyPtrOf<Vec4<u32>>,
}
