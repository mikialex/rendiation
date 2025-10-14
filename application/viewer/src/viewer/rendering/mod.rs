use crate::*;

mod culling;
mod egui;
mod frame_logic;
mod grid_ground;
mod lighting;
mod outline;
mod ray_tracing;
mod transparent;
mod widget;

mod g_buffer;
pub use culling::*;
pub use g_buffer::*;
pub use ray_tracing::*;
pub use transparent::*;

mod post;
pub use frame_logic::*;
use futures::Future;
use grid_ground::*;
pub use lighting::*;
pub use post::*;
use rendiation_oit::*;
use rendiation_scene_rendering_gpu_indirect::*;
use rendiation_scene_rendering_gpu_ray_tracing::*;
use rendiation_texture_gpu_process::copy_frame;
use rendiation_webgpu::*;
use rendiation_webgpu_virtual_buffer::*;
use widget::*;

#[derive(Serialize, Deserialize)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RasterizationRenderBackendType {
  Gles,
  Indirect,
}

#[derive(Clone, Copy, Hash)]
pub struct ViewerNDC {
  pub enable_reverse_z: bool,
}

/// currently, the reverse z is implement by a custom ndc space mapper.
/// this is conceptually wrong because ndc is not changed at all.
/// however it's convenient to do so because the reverse operation must implement in projection(not post transform)
/// and ndc space mapper create a good place to inject projection modification logic.
impl<T: Scalar> NDCSpaceMapper<T> for ViewerNDC {
  fn transform_from_opengl_standard_ndc(&self) -> Mat4<T> {
    let mut m = WebGPUxNDC.transform_from_opengl_standard_ndc();

    if self.enable_reverse_z {
      m.c3 = -T::half()
    }
    m
  }
}

struct ViewerRendererInstance {
  camera: CameraRenderer,
  background: SceneBackgroundRenderer,
  raster_scene_renderer: Box<dyn SceneRenderer>,
  extractor: DefaultSceneBatchExtractor,
  rtx_renderer: Option<(RayTracingRendererGroup, RtxSystemCore)>,
  lighting: LightingRenderingCxPrepareCtx,
  culling: ViewerCulling,
  oit: ViewerTransparentRenderer,
  mesh_lod_graph_renderer: Option<MeshLODGraphSceneRenderer>,
  camera_transforms: BoxedDynQuery<EntityHandle<SceneCameraEntity>, CameraTransform>,
  sm_world_bounding: BoxedDynQuery<EntityHandle<SceneModelEntity>, Box3<f64>>,
}

pub struct Viewer3dRenderingCtx {
  frame_index: u64,
  pub(crate) ndc: ViewerNDC,
  frame_logic: ViewerFrameLogic,
  enable_indirect_occlusion_culling: bool,
  using_host_driven_indirect_draw: bool,
  current_renderer_impl_ty: RasterizationRenderBackendType,
  rtx_effect_mode: RayTracingEffectMode,
  rtx_renderer_enabled: bool,
  rtx_rendering_enabled: bool,
  request_reset_rtx_sample: bool,
  lighting: LightSystem,
  transparent_config: ViewerTransparentContentRenderStyle,
  pool: AttachmentPool,
  gpu: GPU,
  swap_chain: ApplicationWindowSurface,
  on_encoding_finished: EventSource<ViewRenderedState>,
  expect_read_back_for_next_render_result: bool,
  pub picker: GPUxEntityIdMapPicker,
  pub statistics: FramePassStatistics,
  pub enable_statistic_collect: bool,
  prefer_bindless_for_indirect_texture_system: bool,

  stat_frame_time_in_ms: StatisticStore<f32>,
  last_render_timestamp: Option<Instant>,

  pub(crate) init_config: ViewerStaticInitConfig,
}

impl Viewer3dRenderingCtx {
  pub fn setup_init_config(&self, init_config: &mut ViewerInitConfig) {
    init_config.raster_backend_type = self.current_renderer_impl_ty;
    init_config.enable_indirect_occlusion_culling = self.enable_indirect_occlusion_culling;
    init_config.transparent_config = self.transparent_config;
    init_config.prefer_bindless_for_indirect_texture_system =
      self.prefer_bindless_for_indirect_texture_system;
    init_config.init_only = self.init_config.clone();
    init_config.present_mode = self.swap_chain.internal(|v| v.config.present_mode);
  }

  pub fn gpu(&self) -> &GPU {
    &self.gpu
  }

  pub fn tick_frame(&mut self) {
    self.pool.tick();
  }

  pub fn new(
    gpu: GPU,
    swap_chain: ApplicationWindowSurface,
    ndc: ViewerNDC,
    init_config: &ViewerInitConfig,
  ) -> Self {
    Self {
      prefer_bindless_for_indirect_texture_system: init_config
        .prefer_bindless_for_indirect_texture_system,
      enable_statistic_collect: false,
      using_host_driven_indirect_draw: init_config.using_host_driven_indirect_draw,
      frame_index: 0,
      ndc,
      swap_chain,
      enable_indirect_occlusion_culling: init_config.enable_indirect_occlusion_culling,
      transparent_config: init_config.transparent_config,
      current_renderer_impl_ty: init_config.raster_backend_type,
      rtx_effect_mode: RayTracingEffectMode::ReferenceTracing,
      rtx_rendering_enabled: false,
      rtx_renderer_enabled: false,
      request_reset_rtx_sample: false,
      frame_logic: ViewerFrameLogic::new(&gpu),
      lighting: LightSystem::new(&gpu),
      pool: init_attachment_pool(&gpu),
      statistics: FramePassStatistics::new(64, &gpu),
      gpu,
      on_encoding_finished: Default::default(),
      expect_read_back_for_next_render_result: false,
      picker: Default::default(),
      stat_frame_time_in_ms: StatisticStore::new(200),
      last_render_timestamp: Default::default(),
      init_config: init_config.init_only.clone(),
    }
  }

  fn use_viewer_scene_renderer(
    &mut self,
    cx: &mut QueryGPUHookCx,
  ) -> Option<ViewerRendererInstance> {
    let (cx, change_scope) = cx.use_begin_change_set_collect();

    let camera = use_camera_uniforms(cx, self.ndc);
    let background = use_background(cx);

    let ty = get_suitable_texture_system_ty(
      cx.gpu,
      matches!(
        self.current_renderer_impl_ty,
        RasterizationRenderBackendType::Indirect
      ) || self.rtx_renderer_enabled,
      self.prefer_bindless_for_indirect_texture_system,
    );
    let texture_sys = use_texture_system(cx, ty, &self.init_config.texture_pool_source_init_config);

    let any_base_resource_changed = change_scope(cx);
    let mut any_indirect_resource_changed = None;
    let mut rtx_materials_support = None;
    let mut rtx_mesh = None;

    let t_clone = texture_sys.clone();
    let attributes_custom_key = Arc::new(|_: u32, _: &mut _| {}) as Arc<_>;

    let is_indirect = self.current_renderer_impl_ty == RasterizationRenderBackendType::Indirect;
    let culling = use_viewer_culling(
      cx,
      self.ndc,
      self.enable_indirect_occlusion_culling,
      is_indirect,
    );

    let mut mesh_lod_graph_renderer = None;

    let raster_scene_renderer = match self.current_renderer_impl_ty {
      RasterizationRenderBackendType::Gles => cx.scope(|cx| {
        use_gles_scene_renderer(
          cx,
          self.ndc.enable_reverse_z,
          attributes_custom_key,
          t_clone,
        )
        .map(|r| Box::new(r) as Box<dyn SceneRenderer>)
      }),
      RasterizationRenderBackendType::Indirect => cx.scope(|cx| {
        let (cx, change_scope) = cx.use_begin_change_set_collect();

        let enable_combine = self.init_config.enable_indirect_storage_combine;

        let scope = use_readonly_storage_buffer_combine(cx, "indirect materials", enable_combine);
        let unlit_material = use_unlit_material_storage(cx);
        let pbr_mr_material = use_pbr_mr_material_storage(cx);
        let pbr_sg_material = use_pbr_sg_material_storage(cx);
        scope.end(cx);

        let scope = use_readonly_storage_buffer_combine(cx, "indirect mesh", enable_combine);

        let merge_with_vertex_allocator = self
          .init_config
          .using_texture_as_storage_buffer_for_indirect_rendering;

        let mesh = use_bindless_mesh(
          cx,
          &self.init_config.bindless_mesh_init,
          merge_with_vertex_allocator,
        );

        scope.end(cx);

        if self.rtx_renderer_enabled {
          rtx_materials_support = cx.when_render(|| {
            Arc::new(vec![
              Box::new(pbr_mr_material.clone().unwrap()) as Box<dyn SceneMaterialSurfaceSupport>,
              Box::new(pbr_sg_material.clone().unwrap()) as Box<dyn SceneMaterialSurfaceSupport>,
            ])
          });
        }

        let materials = cx.when_render(|| {
          Box::new(vec![
            Box::new(unlit_material.unwrap()) as Box<dyn IndirectModelMaterialRenderImpl>,
            Box::new(pbr_mr_material.unwrap()),
            Box::new(pbr_sg_material.unwrap()),
          ]) as Box<dyn IndirectModelMaterialRenderImpl>
        });

        if self.rtx_renderer_enabled {
          rtx_mesh = mesh.clone();
        }

        mesh_lod_graph_renderer = use_mesh_lod_graph_scene_renderer(cx);

        let mesh = cx.when_render(|| {
          Box::new(vec![
            Box::new(mesh.unwrap()) as Box<dyn IndirectModelShapeRenderImpl>,
            Box::new(mesh_lod_graph_renderer.clone().unwrap()),
          ]) as Box<dyn IndirectModelShapeRenderImpl>
        });

        let std_model = use_std_model_renderer(cx, materials, mesh);
        let scene_model = use_indirect_scene_model(cx, std_model.map(|v| Box::new(v) as Box<_>));

        let renderer = cx
          .when_render(|| IndirectSceneRenderer {
            texture_system: t_clone.unwrap(),
            renderer: scene_model.map(|v| Box::new(v) as Box<_>).unwrap(),
            reversed_depth: self.ndc.enable_reverse_z,
            using_host_driven_indirect_draw: self.using_host_driven_indirect_draw,
          })
          .map(|r| Box::new(r) as Box<dyn SceneRenderer>);

        any_indirect_resource_changed = change_scope(cx);

        renderer
      }),
    };

    let lighting = use_lighting(cx, self.ndc);

    let rtx_scene_renderer = if self.rtx_renderer_enabled {
      cx.scope(|cx| {
        // when indirect raster render is not enabled, we create necessary resource by ourself.
        if self.current_renderer_impl_ty == RasterizationRenderBackendType::Gles {
          cx.scope(|cx| {
            let (cx, change_scope) = cx.use_begin_change_set_collect();

            let limits = &cx.gpu.info.supported_limits;
            let enable_combine = limits.max_storage_buffers_per_shader_stage <= 128;

            let scope = use_readonly_storage_buffer_combine(cx, "rtx materials", enable_combine);
            let pbr_mr_material = use_pbr_mr_material_storage(cx);
            let pbr_sg_material = use_pbr_sg_material_storage(cx);
            scope.end(cx);

            let scope = use_readonly_storage_buffer_combine(cx, "indirect mesh", enable_combine);
            let mesh = use_bindless_mesh(cx, &self.init_config.bindless_mesh_init, false);
            scope.end(cx);

            any_indirect_resource_changed = change_scope(cx);

            rtx_materials_support = cx.when_render(|| {
              Arc::new(vec![
                Box::new(pbr_mr_material.clone().unwrap()) as Box<dyn SceneMaterialSurfaceSupport>,
                Box::new(pbr_sg_material.clone().unwrap()) as Box<dyn SceneMaterialSurfaceSupport>,
              ])
            });

            rtx_mesh = mesh.clone();
          });
        }

        let camera = camera
          .clone()
          .map(|c| Box::new(c) as Box<dyn RtxCameraRenderImpl>);

        let request_reset_sample = self.request_reset_rtx_sample
          || any_base_resource_changed.unwrap_or(false)
          || any_indirect_resource_changed.unwrap_or(false);

        use_viewer_rtx(
          cx,
          camera,
          rtx_materials_support,
          rtx_mesh,
          texture_sys,
          request_reset_sample,
        )
      })
    } else {
      None
    };

    self.request_reset_rtx_sample = false;

    let oit = match self.transparent_config {
      ViewerTransparentContentRenderStyle::NaiveAlphaBlend => {
        ViewerTransparentRenderer::NaiveAlphaBlend
      }
      ViewerTransparentContentRenderStyle::Loop32OIT => cx.scope(|cx| {
        let (_, r) = cx.use_sharable_plain_state(|| OitLoop32Renderer::new(4));
        ViewerTransparentRenderer::Loop32OIT(r.clone())
      }),
      ViewerTransparentContentRenderStyle::WeightedOIT => ViewerTransparentRenderer::WeightedOIT,
    };

    let extractor = use_default_scene_batch_extractor(cx);

    let camera_transforms = cx
      .use_shared_dual_query_view(GlobalCameraTransformShare(self.ndc))
      .use_assure_result(cx);

    let sm_world_bounding = cx
      .use_shared_dual_query_view(SceneModelWorldBounding)
      .use_assure_result(cx);

    cx.when_render(|| ViewerRendererInstance {
      camera: camera.unwrap(),
      background: background.unwrap(),
      extractor: extractor.unwrap(),
      raster_scene_renderer: raster_scene_renderer.unwrap(),
      rtx_renderer: rtx_scene_renderer,
      lighting: lighting.unwrap(),
      culling: culling.unwrap(),
      oit,
      mesh_lod_graph_renderer,
      camera_transforms: camera_transforms
        .expect_resolve_stage()
        .mark_entity_type()
        .into_boxed(),
      sm_world_bounding: sm_world_bounding
        .expect_resolve_stage()
        .mark_entity_type()
        .into_boxed(),
    })
  }

  /// only texture could be read. caller must sure the target passed in render call not using
  /// window surface.
  #[allow(unused)] // used in terminal command
  pub fn read_next_render_result(
    &mut self,
  ) -> impl Future<Output = Result<ReadableTextureBuffer, ViewerRenderResultReadBackErr>> {
    self.expect_read_back_for_next_render_result = true;
    use futures::FutureExt;
    self
      .on_encoding_finished
      .once_future(|result| result.clone().read())
      .flatten()
  }

  fn storage_allocator(&self) -> Box<dyn AbstractStorageAllocator> {
    if self
      .init_config
      .using_texture_as_storage_buffer_for_indirect_rendering
    {
      Box::new(rendiation_webgpu_texture_as_buffer::TextureAsStorageAllocator(self.gpu.clone()))
    } else {
      Box::new(DefaultStorageAllocator)
    }
  }

  pub fn update_registry(
    &mut self,
    memory: &mut FunctionMemory,
    task_spawner: &TaskSpawner,
    shared_ctx: &mut SharedHooksCtx,
  ) -> AsyncTaskPool {
    let mut pool = AsyncTaskPool::default();
    let gpu = self.gpu.clone();

    shared_ctx.reset_visiting();
    QueryGPUHookCx {
      memory,
      gpu: &gpu,
      stage: GPUQueryHookStage::Update {
        spawner: task_spawner,
        task_pool: &mut pool,
        change_collector: &mut Default::default(),
      },
      shared_ctx,
      storage_allocator: self.storage_allocator(),
    }
    .execute(|cx| self.use_viewer_scene_renderer(cx), true);
    pool
  }

  pub fn inspect(
    &mut self,
    memory: &mut FunctionMemory,
    shared_ctx: &mut SharedHooksCtx,
    inspector: &mut dyn Inspector,
  ) {
    if !memory.created {
      return;
    }

    let gpu = self.gpu.clone();
    shared_ctx.reset_visiting();
    QueryGPUHookCx {
      memory,
      gpu: &gpu,
      stage: GPUQueryHookStage::Inspect(inspector),
      shared_ctx,
      storage_allocator: self.storage_allocator(),
    }
    .execute(|cx| self.use_viewer_scene_renderer(cx), true);
  }

  #[instrument(name = "frame rendering", skip_all)]
  pub fn render(
    &mut self,
    target: &RenderTargetView,
    content: &Viewer3dContent,
    memory: &mut FunctionMemory,
    task_pool_result: TaskPoolResultCx,
    shared_ctx: &mut SharedHooksCtx,
  ) {
    let gpu = self.gpu.clone();
    shared_ctx.reset_visiting();
    let renderer = QueryGPUHookCx {
      memory,
      gpu: &gpu,
      stage: GPUQueryHookStage::CreateRender {
        task: task_pool_result,
      },
      shared_ctx,
      storage_allocator: self.storage_allocator(),
    }
    .execute(|cx| self.use_viewer_scene_renderer(cx).unwrap(), true);

    self.frame_index += 1;
    let now = Instant::now();
    if let Some(last_frame_time) = self.last_render_timestamp.take() {
      self.stat_frame_time_in_ms.insert(
        now.duration_since(last_frame_time).as_secs_f32() * 1000.,
        self.frame_index,
      );
    }
    self.last_render_timestamp = Some(now);

    let statistics = self
      .enable_statistic_collect
      .then(|| self.statistics.create_resolver(self.frame_index));

    let mut ctx = FrameCtx::new(&self.gpu, target.size(), &self.pool, statistics);

    let lighting_cx = self.lighting.prepare(
      renderer.lighting,
      &mut ctx,
      self.ndc.enable_reverse_z,
      renderer.raster_scene_renderer.as_ref(),
      &renderer.extractor,
      content.scene,
    );

    let render_target = if self.expect_read_back_for_next_render_result
      && matches!(target, RenderTargetView::SurfaceTexture { .. })
    {
      // we do extra copy in this case, so we have to make sure the copy source has correct usage
      let mut key = target.create_attachment_key();
      key.usage |= TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC;
      key.request(&ctx)
    } else {
      target.clone()
    };

    if self.rtx_rendering_enabled {
      if let Some((rtx_renderer, core)) = &renderer.rtx_renderer {
        match self.rtx_effect_mode {
          RayTracingEffectMode::AO => {
            let ao_result = rtx_renderer.ao.render(
              &mut ctx,
              core.rtx_system.as_ref(),
              &rtx_renderer.base,
              content.scene,
              content.main_camera,
            );

            pass("copy rtx ao into final target")
              .with_color(target, store_full_frame())
              .render_ctx(&mut ctx)
              .by(&mut copy_frame(RenderTargetView::from(ao_result), None));
          }
          RayTracingEffectMode::ReferenceTracing => {
            let result = rtx_renderer.pt.render(
              &mut ctx,
              core.rtx_system.as_ref(),
              &rtx_renderer.base,
              content.scene,
              content.main_camera,
              &self.lighting.tonemap,
              &renderer.background,
            );
            pass("copy pt result into final target")
              .with_color(target, store_full_frame())
              .render_ctx(&mut ctx)
              .by(&mut copy_frame(RenderTargetView::from(result), None));
          }
        }

        self.picker.notify_frame_id_buffer_not_available();
      }
    } else {
      let ras_renderer = ViewerSceneRenderer {
        scene: renderer.raster_scene_renderer.as_ref(),
        batch_extractor: &renderer.extractor,
        cameras: &renderer.camera,
        background: &renderer.background,
        reversed_depth: self.ndc.enable_reverse_z,
        oit: renderer.oit.clone(),
        camera_transforms: &renderer.camera_transforms,
        sm_world_bounding: &renderer.sm_world_bounding,
      };

      let camera_transform = ras_renderer
        .camera_transforms
        .access(&content.main_camera)
        .unwrap();
      let current_view_projection_inv = camera_transform.view_projection_inv;

      if let Some(mesh_lod_graph_renderer) = &renderer.mesh_lod_graph_renderer {
        if camera_transform
          .projection
          .check_is_perspective_matrix_assume_common_projection()
        {
          mesh_lod_graph_renderer.setup_lod_decider(
            &gpu,
            camera_transform.projection,
            camera_transform.world,
            render_target.size().into_f32().into(),
          );
        }
      }

      let entity_id = self.frame_logic.render(
        &mut ctx,
        &ras_renderer,
        &renderer.culling,
        &lighting_cx,
        content,
        &render_target,
        current_view_projection_inv,
        self.ndc.enable_reverse_z,
      );

      let entity_id = entity_id.expect_standalone_common_texture_view();
      self.picker.read_new_frame_id_buffer(
        &GPUTypedTextureView::<TextureDimension2, u32>::try_from(entity_id.clone()).unwrap(),
        &self.gpu,
        &mut ctx.encoder,
      );
      //
    }

    {
      let main_camera_gpu = renderer.camera.make_component(content.main_camera).unwrap();

      let widgets_result = draw_widgets(
        &mut ctx,
        renderer.raster_scene_renderer.as_ref(),
        &renderer.extractor,
        content.widget_scene,
        self.ndc.enable_reverse_z,
        &main_camera_gpu,
        &self.frame_logic.axis,
      );
      let mut copy_scene_msaa_widgets = copy_frame(
        widgets_result,
        BlendState::PREMULTIPLIED_ALPHA_BLENDING.into(),
      );
      pass("copy_scene_msaa_widgets")
        .with_color(&render_target, load_and_store())
        .render_ctx(&mut ctx)
        .by(&mut copy_scene_msaa_widgets);
    }

    // do extra copy to surface texture
    if self.expect_read_back_for_next_render_result
      && matches!(target, RenderTargetView::SurfaceTexture { .. })
    {
      pass("extra final copy to surface")
        .with_color(target, store_full_frame())
        .render_ctx(&mut ctx)
        .by(&mut rendiation_texture_gpu_process::copy_frame(
          render_target.clone(),
          None,
        ));
    }
    self.expect_read_back_for_next_render_result = false;
    drop(ctx);

    noop_ctx!(cx);
    self.statistics.poll(cx);

    self.on_encoding_finished.emit(&ViewRenderedState {
      target: render_target,
      device: self.gpu.device.clone(),
      queue: self.gpu.queue.clone(),
    });
  }
}

#[derive(Clone)]
struct ViewRenderedState {
  target: RenderTargetView,
  device: GPUDevice,
  queue: GPUQueue,
}

#[derive(Debug)]
pub enum ViewerRenderResultReadBackErr {
  Gpu(rendiation_webgpu::BufferAsyncError),
  UnableToReadSurfaceTexture,
}

impl ViewRenderedState {
  async fn read(self) -> Result<ReadableTextureBuffer, ViewerRenderResultReadBackErr> {
    let tex = match self.target {
      RenderTargetView::Texture(tex) => tex.clone(),
      RenderTargetView::ReusedTexture(tex) => tex.item().clone(),
      RenderTargetView::SurfaceTexture { .. } => {
        // note: the usage of surface texture could only contains TEXTURE_BINDING, so it's impossible
        // to do any read back from it. the upper layer should be draw content into temp texture for read back
        // and copy back to surface.
        return Err(ViewerRenderResultReadBackErr::UnableToReadSurfaceTexture);
      }
    };

    let mut encoder = self.device.create_encoder();

    let tex = GPU2DTextureView::try_from(tex).unwrap();

    let buffer = encoder.read_texture_2d::<f32>(
      &self.device,
      &tex,
      ReadRange {
        size: tex.size(),
        offset_x: 0,
        offset_y: 0,
      },
    );
    self.queue.submit_encoder(encoder);

    buffer.await.map_err(ViewerRenderResultReadBackErr::Gpu)
  }
}
