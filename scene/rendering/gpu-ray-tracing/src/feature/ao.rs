use rendiation_shader_library::sampling::{hammersley_2d_fn, sample_hemisphere_cos_fn, tbn_fn};
use rendiation_texture_core::Size;
use rendiation_webgpu_reactive_utils::{CommonStorageBufferImpl, ReactiveStorageBufferContainer};

use crate::*;

pub struct RayTracingAORenderSystem {
  camera: DefaultRtxCameraRenderImplProvider,
  sbt: UpdateResultToken,
  executor: GPURaytracingPipelineExecutor,
  scene_tlas: UpdateResultToken, // todo, share, unify the share mechanism with the texture
  mesh: MeshBindlessGPUSystemSource, // todo, share
  sm_to_mesh: UpdateResultToken, // todo share?
  system: RtxSystemCore,
  ao_buffer: Arc<RwLock<Option<GPU2DTextureView>>>,
  shader_handles: AOShaderHandles,
}

#[derive(Clone, PartialEq, Debug)]
struct AOShaderHandles {
  ray_gen: ShaderHandle,
  closest_hit: ShaderHandle,
  any_hit: ShaderHandle,
  miss: ShaderHandle,
}

impl RayTracingAORenderSystem {
  pub fn new(rtx: &RtxSystemCore, gpu: &GPU) -> Self {
    Self {
      camera: Default::default(),
      scene_tlas: Default::default(),
      sbt: Default::default(),
      sm_to_mesh: Default::default(),
      executor: rtx.rtx_device.create_raytracing_pipeline_executor(),
      system: rtx.clone(),
      ao_buffer: Default::default(),
      mesh: MeshBindlessGPUSystemSource::new(gpu),
      shader_handles: AOShaderHandles {
        ray_gen: ShaderHandle(0, RayTracingShaderStage::RayGeneration),
        closest_hit: ShaderHandle(0, RayTracingShaderStage::ClosestHit),
        any_hit: ShaderHandle(0, RayTracingShaderStage::AnyHit),
        miss: ShaderHandle(0, RayTracingShaderStage::Miss),
      },
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

    let sm_to_mesh = ReactiveStorageBufferContainer::<u32>::new(cx).with_source(
      global_watch()
        .watch_typed_foreign_key::<StandardModelRefAttributesMeshEntity>()
        .collective_filter_map(|v| v)
        .one_to_many_fanout(global_rev_ref().watch_inv_ref::<SceneModelStdModelRenderPayload>())
        .collective_map(|v| v.alloc_index())
        .into_boxed(),
      0,
    );

    self.sm_to_mesh = source.register_multi_updater(sm_to_mesh.inner);

    self.sbt = source.register_multi_updater(sbt);
    self.camera.register_resource(source, cx);
    self.mesh.register_resource(source, cx);
  }

  fn deregister_resource(&mut self, source: &mut ReactiveQueryJoinUpdater) {
    source.deregister(&mut self.scene_tlas);
    source.deregister(&mut self.sbt);
    source.deregister(&mut self.sm_to_mesh);
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
      ao_buffer: self.ao_buffer.clone(),
      mesh: self.mesh.create_impl_internal_impl(res),
      sm_to_mesh: res
        .take_multi_updater_updated::<CommonStorageBufferImpl<u32>>(self.sm_to_mesh)
        .unwrap()
        .gpu()
        .clone(),
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
  ao_buffer: Arc<RwLock<Option<GPU2DTextureView>>>,
  mesh: MeshGPUBindlessImpl,
  sm_to_mesh: StorageBufferReadOnlyDataView<[u32]>,
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
  pub fn render(
    &mut self,
    frame: &mut FrameCtx,
    scene: EntityHandle<SceneEntity>,
    camera: EntityHandle<SceneCameraEntity>,
  ) -> GPU2DTextureView {
    let scene_tlas = self.scene_tlas.access(&scene).unwrap().clone();
    // let render_size = frame.frame_size();
    let render_size = Size::from_u32_pair_min_one((64, 64));

    let mut ao_buffer = self.ao_buffer.write();
    let ao_buffer = ao_buffer.deref_mut();
    if let Some(ao) = &ao_buffer {
      if ao.size() != render_size {
        *ao_buffer = None;
      }
    }

    let ao_buffer = ao_buffer
      .get_or_insert_with(|| {
        create_empty_2d_texture_view(
          frame.gpu,
          render_size,
          TextureUsages::all(),
          TextureFormat::Rgba8Unorm,
        )
      })
      .clone();
    let ao_buffer_rw = ao_buffer
      .clone()
      .into_storage_texture_view_readwrite()
      .unwrap();

    let mut desc = GPURaytracingPipelineAndBindingSource::default();

    let camera = self.camera.get_rtx_camera(camera);

    let trace_base_builder = self.rtx_system.create_tracer_base_builder();

    let ray_gen_shader = trace_base_builder
      .create_ray_gen_shader_base()
      .inject_ctx(RayTracingAORayGenCtx {
        camera,
        ao_buffer: ao_buffer_rw,
        scene: scene_tlas.clone(),
      })
      .then_trace(|_, ctx| {
        let rg_cx = ctx.expect_ray_gen_ctx();
        let cx = ctx.expect_custom_cx::<RayTracingAORayGenCtxInvocation>();
        let normalized_position =
          rg_cx.launch_id().into_f32().xy() / rg_cx.launch_size().into_f32().xy();
        let ray = cx.camera.generate_ray(normalized_position);

        let trace_call = ShaderRayTraceCall {
          tlas_idx: cx.tlas_idx,
          ray_flags: val(RayFlagConfigRaw::RAY_FLAG_CULL_BACK_FACING_TRIANGLES as u32),
          cull_mask: val(u32::MAX),
          sbt_ray_config: AORayType::Primary.to_sbt_cfg(),
          miss_index: val(0),
          ray,
          range: ShaderRayRange::default(),
        };

        (val(true), trace_call, val(0.))
      })
      .map(|(_, payload), ctx| {
        let cx = ctx.expect_custom_cx::<RayTracingAORayGenCtxInvocation>();
        let position = ctx.expect_ray_gen_ctx().launch_id().xy();

        cx.ao_buffer
          .write_texel(position, (payload, payload, payload, val(1.0)).into());
      });

    type RayGenTracePayload = f32; // occlusion
    let bindless_mesh = self.mesh.make_bindless_dispatcher();
    let ao_closest = RayTracingAOComputeTraceOperator {
      base: trace_base_builder.create_closest_hit_shader_base::<RayGenTracePayload>(),
      scene: scene_tlas,
      max_sample_count: 8,
      bindless_mesh,
      sm_to_mesh: self.sm_to_mesh.clone(),
    };

    let handles = AOShaderHandles {
      ray_gen: desc.register_ray_gen::<u32>(ShaderFutureProviderIntoTraceOperator(ray_gen_shader)),
      closest_hit: desc.register_ray_closest_hit::<RayGenTracePayload>(
        ShaderFutureProviderIntoTraceOperator(ao_closest),
      ),
      any_hit: desc.register_ray_any_hit(|_any_ctx| {
        val(ANYHIT_BEHAVIOR_ACCEPT_HIT & ANYHIT_BEHAVIOR_END_SEARCH)
      }),
      miss: desc.register_ray_miss::<RayGenTracePayload>(
        trace_base_builder
          .create_miss_hit_shader_base::<RayGenTracePayload>()
          .map(|_, cx| {
            cx.payload().unwrap().store(val(1.0_f32));
          }),
      ),
    };
    assert_eq!(handles, self.shader_handles);

    let mut rtx_encoder = self.rtx_system.create_raytracing_encoder();

    let sbt = self.sbt.inner.read();
    rtx_encoder.trace_ray(
      &desc,
      &self.executor,
      (
        render_size.width_usize() as u32,
        render_size.height_usize() as u32,
        1,
      ),
      (*sbt).as_ref(),
    );

    ao_buffer
  }
}

#[derive(Clone)]
struct RayTracingAORayGenCtx {
  camera: Box<dyn RtxCameraRenderComponent>,
  ao_buffer: StorageTextureReadWrite<GPU2DTextureView>,
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
    }
  }

  fn bind(&self, builder: &mut BindingBuilder) {
    self.camera.bind(builder);
    builder.bind(&self.ao_buffer);
    self.scene.bind(builder);
  }
}

#[derive(Clone)]
struct RayTracingAORayGenCtxInvocation {
  camera: Box<dyn RtxCameraRenderInvocation>,
  ao_buffer: HandleNode<ShaderStorageTextureRW2D>,
  tlas_idx: Node<u32>,
}

#[derive(Clone)]
struct RayTracingAOComputeTraceOperator {
  base: Box<dyn TraceOperator<()>>,
  max_sample_count: u32,
  scene: TlASInstance,
  bindless_mesh: BindlessMeshDispatcher,
  sm_to_mesh: StorageBufferReadOnlyDataView<[u32]>,
}

impl ShaderHashProvider for RayTracingAOComputeTraceOperator {
  shader_hash_type_id! {}
}

impl ShaderFutureProvider for RayTracingAOComputeTraceOperator {
  type Output = ();
  fn build_device_future(&self, ctx: &mut AnyMap) -> DynShaderFuture<Self::Output> {
    RayTracingAOComputeFuture {
      upstream: self.base.build_device_future(ctx),
      max_sample_count: self.max_sample_count,
      tracing: TracingFuture::default(),
      bindless_mesh: self.bindless_mesh.clone(),
      tlas: self.scene.clone(),
      sm_to_mesh: self.sm_to_mesh.clone(),
    }
    .into_dyn()
  }
}

struct RayTracingAOComputeFuture {
  upstream: DynShaderFuture<()>,
  max_sample_count: u32,
  tracing: TracingFuture<u32>,
  bindless_mesh: BindlessMeshDispatcher,
  sm_to_mesh: StorageBufferReadOnlyDataView<[u32]>,
  tlas: TlASInstance,
}

impl ShaderFuture for RayTracingAOComputeFuture {
  type Output = ();

  type Invocation = RayTracingAOComputeInvocation;

  fn required_poll_count(&self) -> usize {
    self.upstream.required_poll_count() + 1
  }

  fn build_poll(&self, ctx: &mut DeviceTaskSystemBuildCtx) -> Self::Invocation {
    RayTracingAOComputeInvocation {
      upstream: self.upstream.build_poll(ctx),
      max_sample_count: self.max_sample_count,
      hit_position: ctx.make_state::<Node<Vec3<f32>>>(),
      hit_normal_tbn: ctx.make_state::<Node<Mat3<f32>>>(),
      next_sample_idx: ctx.make_state::<Node<u32>>(),
      occlusion_count: ctx.make_state::<Node<u32>>(),
      trace_on_the_fly: self.tracing.build_poll(ctx),
      bindless_mesh: self
        .bindless_mesh
        .build_bindless_mesh_rtx_access(ctx.compute_cx.bindgroups()),
      sm_to_mesh: ctx.compute_cx.bind_by(&self.sm_to_mesh),
      tlas: self.tlas.build(ctx.compute_cx.bindgroups()),
    }
  }

  fn bind_input(&self, builder: &mut DeviceTaskSystemBindCtx) {
    self.upstream.bind_input(builder);
    self.tracing.bind_input(builder);
    self.bindless_mesh.bind_bindless_mesh_rtx_access(builder);
    builder.bind(&self.sm_to_mesh);
    self.tlas.bind(builder);
  }
}

struct RayTracingAOComputeInvocation {
  upstream: Box<dyn ShaderFutureInvocation<Output = ()>>,
  max_sample_count: u32,
  hit_position: BoxedShaderLoadStore<Node<Vec3<f32>>>,
  hit_normal_tbn: BoxedShaderLoadStore<Node<Mat3<f32>>>,
  next_sample_idx: BoxedShaderLoadStore<Node<u32>>,
  occlusion_count: BoxedShaderLoadStore<Node<u32>>,
  trace_on_the_fly: TracingFutureInvocation<u32>, // 0 means not hit
  bindless_mesh: BindlessMeshRtxAccessInvocation,
  sm_to_mesh: ReadOnlyStorageNode<[u32]>,
  tlas: Node<u32>,
}

impl ShaderFutureInvocation for RayTracingAOComputeInvocation {
  type Output = ();

  fn device_poll(&self, ctx: &mut DeviceTaskSystemPollCtx) -> ShaderPoll<Self::Output> {
    let source = self.upstream.device_poll(ctx);

    if_by(ctx.is_fallback_task().not(), || {
      source.resolved.store(true);
    });

    ctx
      .invocation_registry
      .get::<TracingCtx>()
      .unwrap()
      .payload()
      .unwrap()
      .store(val(0.0_f32));

    (val(true).make_local_var(), ()).into()

    // let current_idx = self.next_sample_idx.abstract_load();
    // let sample_is_done = current_idx.greater_equal_than(self.max_sample_count);

    // if_by(current_idx.equals(0), || {
    //   let closest_hit_ctx = ctx
    //     .invocation_registry
    //     .get::<TracingCtx>()
    //     .unwrap()
    //     .closest_hit_ctx()
    //     .unwrap();

    //   let hit_position = closest_hit_ctx.world_ray().origin
    //     + closest_hit_ctx.world_ray().direction * closest_hit_ctx.hit_distance();

    //   let scene_model_id = closest_hit_ctx.instance_custom_id();
    //   let mesh_id = self.sm_to_mesh.index(scene_model_id).load();
    //   let tri_id = closest_hit_ctx.primitive_id();
    //   let tri_idx_s = self.bindless_mesh.get_triangle_idx(tri_id, mesh_id);

    //   let tri_a_normal = self.bindless_mesh.get_normal(tri_idx_s.x(), mesh_id);
    //   let tri_b_normal = self.bindless_mesh.get_normal(tri_idx_s.y(), mesh_id);
    //   let tri_c_normal = self.bindless_mesh.get_normal(tri_idx_s.z(), mesh_id);

    //   let attribs: Node<Vec2<f32>> = closest_hit_ctx.hit_attribute().expand().bary_coord;
    //   let barycentric: Node<Vec3<f32>> = (
    //     val(1.0) - attribs.x() - attribs.y(),
    //     attribs.x(),
    //     attribs.y(),
    //   )
    //     .into();

    //   // Computing the normal at hit position
    //   let normal = tri_a_normal * barycentric.x()
    //     + tri_b_normal * barycentric.y()
    //     + tri_c_normal * barycentric.z();
    //   // Transforming the normal to world space
    //   let normal = (closest_hit_ctx.object_to_world().shrink_to_3() * normal).normalize();

    //   self.hit_position.abstract_store(hit_position);
    //   self.hit_normal_tbn.abstract_store(tbn_fn(normal));
    // });

    // let on_the_fly_trace_not_active = self
    //   .trace_on_the_fly
    //   .task_not_allocated()
    //   .or(self.trace_on_the_fly.task_has_already_resolved());
    // let should_spawn_new_ray = sample_is_done.not().and(on_the_fly_trace_not_active);

    // if_by(should_spawn_new_ray, || {
    //   let random = hammersley_2d_fn(current_idx, val(self.max_sample_count));

    //   let ray = ShaderRay {
    //     origin: self.hit_position.abstract_load(),
    //     direction: self.hit_normal_tbn.abstract_load() * sample_hemisphere_cos_fn(random),
    //   };

    //   let trace_call = ShaderRayTraceCall {
    //     tlas_idx: self.tlas,
    //     ray_flags: val(RayFlagConfigRaw::RAY_FLAG_CULL_BACK_FACING_TRIANGLES as u32),
    //     cull_mask: val(u32::MAX),
    //     sbt_ray_config: AORayType::AOTest.to_sbt_cfg(),
    //     miss_index: val(0),
    //     ray,
    //     range: ShaderRayRange::default(), // todo, should control max ray length
    //   };

    //   let trace_on_the_fly_right =
    //     ctx.spawn_new_tracing_task(val(true), trace_call, val(0), &self.trace_on_the_fly);

    //   self.trace_on_the_fly.abstract_store(trace_on_the_fly_right); // todo, this is weird, should be improved
    // });

    // storage_barrier(); // todo, how to make this invisible for native rtx?

    // // todo, check poll result of current on the fly trace
    // let should_pool = self
    //   .trace_on_the_fly
    //   .task_not_allocated()
    //   .not()
    //   .and(self.trace_on_the_fly.task_has_already_resolved().not());
    // if_by(should_pool, || {
    //   let r = self.trace_on_the_fly.device_poll(ctx);
    //   if_by(r.is_ready, || {
    //     self
    //       .occlusion_count
    //       .abstract_store(self.occlusion_count.abstract_load() + r.payload);
    //   });
    // });

    // storage_barrier();

    // if_by(sample_is_done, || {
    //   let occlusion =
    //     self.occlusion_count.abstract_load().into_f32() / val(self.max_sample_count as f32);
    //   ctx
    //     .invocation_registry
    //     .get::<TracingCtx>()
    //     .unwrap()
    //     .payload()
    //     .unwrap()
    //     .store(occlusion);
    // });

    // (sample_is_done, ()).into()
  }
}
