#[allow(unused_imports)]
use rendiation_shader_library::sampling::{
  hammersley_2d_fn, random2_fn, sample_hemisphere_cos_fn, sample_hemisphere_uniform_fn, tbn_fn,
};
use rendiation_texture_core::Size;

use crate::*;

pub struct RayTracingAORenderSystem {
  camera: DefaultRtxCameraRenderImplProvider,
  sbt: UpdateResultToken,
  executor: GPURaytracingPipelineExecutor,
  scene_tlas: UpdateResultToken, // todo, share, unify the share mechanism with the texture
  mesh: MeshBindlessGPUSystemSource, // todo, share
  system: RtxSystemCore,
  ao_state: Arc<RwLock<Option<AORenderState>>>,
  shader_handles: AOShaderHandles,
}

impl RayTracingAORenderSystem {
  pub fn reset_ao_sample(&self, gpu: &GPU) {
    if let Some(state) = self.ao_state.write().as_mut() {
      state.reset(gpu);
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
        TextureUsages::all(),
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
  any_hit: ShaderHandle,
  miss: ShaderHandle,
}

impl Default for AOShaderHandles {
  fn default() -> Self {
    Self {
      ray_gen: ShaderHandle(0, RayTracingShaderStage::RayGeneration),
      closest_hit: ShaderHandle(0, RayTracingShaderStage::ClosestHit),
      any_hit: ShaderHandle(0, RayTracingShaderStage::AnyHit),
      miss: ShaderHandle(0, RayTracingShaderStage::Miss),
    }
  }
}

impl RayTracingAORenderSystem {
  pub fn new(rtx: &RtxSystemCore, gpu: &GPU) -> Self {
    Self {
      camera: Default::default(),
      scene_tlas: Default::default(),
      sbt: Default::default(),
      executor: rtx.rtx_device.create_raytracing_pipeline_executor(),
      system: rtx.clone(),
      ao_state: Default::default(),
      mesh: MeshBindlessGPUSystemSource::new(gpu),
      shader_handles: Default::default(),
    }
  }
}

impl RenderImplProvider<SceneRayTracingAORenderer> for RayTracingAORenderSystem {
  fn register_resource(&mut self, source: &mut ReactiveQueryJoinUpdater, cx: &GPU) {
    self.scene_tlas =
      source.register_reactive_query(scene_to_tlas(cx, self.system.rtx_acc.clone()));

    // todo support max mesh count grow
    let sbt = GPUSbt::new(
      self
        .system
        .rtx_device
        .create_sbt(1, 2000, GLOBAL_TLAS_MAX_RAY_STRIDE),
    );
    let closest_hit = self.shader_handles.closest_hit;
    let any = self.shader_handles.any_hit;
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
            closest_hit: None,
            any_hit: Some(any),
            intersection: None,
          }),
      });

    self.sbt = source.register_multi_updater(sbt);
    self.camera.register_resource(source, cx);
    self.mesh.register_resource(source, cx);
  }

  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.scene_tlas);
    source.deregister(&mut self.sbt);
    self.camera.deregister_resource(source);
    self.mesh.deregister_resource(source);
  }

  fn create_impl(&self, res: &mut ConcurrentStreamUpdateResult) -> SceneRayTracingAORenderer {
    let sbt = res.take_multi_updater_updated::<GPUSbt>(self.sbt).unwrap();
    SceneRayTracingAORenderer {
      executor: self.executor.clone(),
      sbt: sbt.target.clone(),
      scene_tlas: res.take_reactive_query_updated(self.scene_tlas).unwrap(),
      camera: self.camera.create_impl(res),
      rtx_system: self.system.rtx_system.clone(),
      ao_state: self.ao_state.clone(),
      mesh: self.mesh.create_impl_internal_impl(res),
      shader_handles: self.shader_handles.clone(),
    }
  }
}

pub struct SceneRayTracingAORenderer {
  camera: Box<dyn RtxCameraRenderImpl>,
  executor: GPURaytracingPipelineExecutor,
  sbt: GPUSbt,
  rtx_system: Box<dyn GPURaytracingSystem>,
  scene_tlas: BoxedDynQuery<EntityHandle<SceneEntity>, TlASInstance>,
  ao_state: Arc<RwLock<Option<AORenderState>>>,
  mesh: MeshGPUBindlessImpl,
  shader_handles: AOShaderHandles,
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
  #[instrument(name = "SceneRayTracingAORenderer rendering", skip_all)]
  pub fn render(
    &mut self,
    frame: &mut FrameCtx,
    scene: EntityHandle<SceneEntity>,
    camera: EntityHandle<SceneCameraEntity>,
  ) -> GPU2DTextureView {
    let scene_tlas = self.scene_tlas.access(&scene).unwrap().clone();
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

    let camera = self.camera.get_rtx_camera(camera);

    let trace_base_builder = self.rtx_system.create_tracer_base_builder();

    let ray_gen_shader = trace_base_builder
      .create_ray_gen_shader_base()
      .inject_ctx(RayTracingAORayGenCtx {
        camera,
        ao_buffer: ao_buffer_rw,
        ao_sample_count: ao_state.sample_count.clone(),
        scene: scene_tlas.clone(),
      })
      .then_trace(|_, ctx| {
        let rg_cx = ctx.expect_ray_gen_ctx();
        let ao_cx = ctx.expect_custom_cx::<RayTracingAORayGenCtxInvocation>();
        let normalized_position =
          rg_cx.launch_id().into_f32().xy() / rg_cx.launch_size().into_f32().xy();
        let ray = ao_cx.camera.generate_ray(normalized_position);

        let trace_call = ShaderRayTraceCall {
          tlas_idx: ao_cx.tlas_idx,
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

        let previous_sample_count = ao_cx.ao_sample_count.load().x().into_f32();
        let all_sample_count = previous_sample_count + val(1.0);

        let previous_sample_acc = ao_cx.ao_buffer.load_texel(position, val(0)).x();
        let new_sample_acc =
          (previous_sample_acc * previous_sample_count + payload) / all_sample_count;
        let new_sample_acc = new_sample_acc.splat::<Vec3<_>>();

        ao_cx
          .ao_buffer
          .write_texel(position, (new_sample_acc, val(1.0)).into());
      });

    type RayGenTracePayload = f32; // occlusion
    let bindless_mesh = self.mesh.make_bindless_dispatcher();
    let ao_closest = trace_base_builder
      .create_closest_hit_shader_base::<RayGenTracePayload>()
      .inject_ctx(RayTracingAORayClosestCtx {
        scene: scene_tlas.clone(),
        bindless_mesh,
        ao_sample_count: ao_state.sample_count.clone(),
      })
      .then_trace(|_, ctx| {
        let ao_cx = ctx.expect_custom_cx::<RayTracingAORayClosestCtxInvocation>();
        let closest_hit_ctx = ctx.expect_closest_hit_ctx();

        let scene_model_id = closest_hit_ctx.instance_custom_id();
        let mesh_id = ao_cx.bindless_mesh.sm_to_mesh.index(scene_model_id).load();
        let tri_id = closest_hit_ctx.primitive_id();
        let tri_idx_s = ao_cx.bindless_mesh.get_triangle_idx(tri_id, mesh_id);

        let tri_a_normal = ao_cx.bindless_mesh.get_normal(tri_idx_s.x(), mesh_id);
        let tri_b_normal = ao_cx.bindless_mesh.get_normal(tri_idx_s.y(), mesh_id);
        let tri_c_normal = ao_cx.bindless_mesh.get_normal(tri_idx_s.z(), mesh_id);

        let attribs: Node<Vec2<f32>> = closest_hit_ctx.hit_attribute().expand().bary_coord;
        let barycentric: Node<Vec3<f32>> = (
          val(1.0) - attribs.x() - attribs.y(),
          attribs.x(),
          attribs.y(),
        )
          .into();

        // Computing the normal at hit position
        let normal = tri_a_normal * barycentric.x()
          + tri_b_normal * barycentric.y()
          + tri_c_normal * barycentric.z();
        // Transforming the normal to world space
        let normal =
          (closest_hit_ctx.world_to_object().shrink_to_3().transpose() * normal).normalize();
        let hit_normal_tbn = tbn_fn(normal);

        let origin = closest_hit_ctx.world_ray().origin
          + closest_hit_ctx.world_ray().direction * closest_hit_ctx.hit_distance();

        let random = hammersley_2d_fn(ao_cx.ao_sample_count.load().x(), val(256));
        // let seed = ao_cx.ao_sample_count.load().x().into_f32();
        // let random = random2_fn((seed, (seed + seed).sin().cos()).into());
        let direction = hit_normal_tbn * sample_hemisphere_cos_fn(random);
        // let direction = hit_normal_tbn * sample_hemisphere_uniform_fn(random);

        let ray = ShaderRay { origin, direction };

        let trace_call = ShaderRayTraceCall {
          tlas_idx: ao_cx.tlas,
          ray_flags: val(RayFlagConfigRaw::RAY_FLAG_CULL_BACK_FACING_TRIANGLES as u32),
          cull_mask: val(u32::MAX),
          sbt_ray_config: AORayType::AOTest.to_sbt_cfg(),
          miss_index: val(0), // using the sample miss shader as primary ray
          ray,
          range: ShaderRayRange {
            min: val(0.01),
            max: val(10.0),
          },
        };

        (val(true), trace_call, val(1.))
      })
      .map(|(_, payload), ctx| ctx.expect_payload().store(payload));

    source.max_in_flight_trace_ray(2);
    let handles = AOShaderHandles {
      ray_gen: source.register_ray_gen(ray_gen_shader),
      closest_hit: source.register_ray_closest_hit::<RayGenTracePayload>(ao_closest, 1),
      any_hit: source.register_ray_any_hit(|any_ctx| {
        any_ctx
          .payload::<RayGenTracePayload>()
          .abstract_store(val(0.0));
        val(ANYHIT_BEHAVIOR_ACCEPT_HIT | ANYHIT_BEHAVIOR_END_SEARCH)
      }),
      miss: source.register_ray_miss::<RayGenTracePayload>(
        trace_base_builder
          .create_miss_hit_shader_base::<RayGenTracePayload>()
          .map(|_, cx| {
            cx.payload().unwrap().store(val(1.0_f32));
          }),
        1,
      ),
    };
    assert_eq!(handles, self.shader_handles);
    source.set_execution_round_hint(8);

    let mut rtx_encoder = self.rtx_system.create_raytracing_encoder();

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
  ao_buffer: StorageTextureReadWrite<GPU2DTextureView>,
  ao_sample_count: UniformBufferDataView<Vec4<u32>>,
  scene: TlASInstance,
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
      tlas_idx: self.scene.build(cx),
      ao_sample_count: cx.bind_by(&self.ao_sample_count),
    }
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    self.camera.bind(builder);
    builder.bind(&self.ao_buffer);
    self.scene.bind(builder);
    builder.bind(&self.ao_sample_count);
  }
}

#[derive(Clone)]
struct RayTracingAORayGenCtxInvocation {
  camera: Box<dyn RtxCameraRenderInvocation>,
  ao_buffer: HandleNode<ShaderStorageTextureRW2D>,
  tlas_idx: Node<u32>,
  ao_sample_count: UniformNode<Vec4<u32>>,
}

#[derive(Clone)]
struct RayTracingAORayClosestCtx {
  scene: TlASInstance,
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
      tlas: self.scene.build(cx),
      bindless_mesh: self.bindless_mesh.build_bindless_mesh_rtx_access(cx),
      ao_sample_count: cx.bind_by(&self.ao_sample_count),
    }
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    self.scene.bind(builder);
    self.bindless_mesh.bind_bindless_mesh_rtx_access(builder);
    builder.bind(&self.ao_sample_count);
  }
}

#[derive(Clone)]
struct RayTracingAORayClosestCtxInvocation {
  bindless_mesh: BindlessMeshRtxAccessInvocation,
  tlas: Node<u32>,
  ao_sample_count: UniformNode<Vec4<u32>>,
}
