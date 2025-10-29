use fast_hash_collection::{FastHashMap, FastHashSet};
use rendiation_scene_rendering_gpu_indirect::*;
use rendiation_scene_rendering_gpu_ray_tracing::*;
use rendiation_webgpu::*;
use rendiation_webgpu_virtual_buffer::*;

use crate::*;

#[derive(Serialize, Deserialize)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RasterizationRenderBackendType {
  Gles,
  Indirect,
}

pub struct Viewer3dRenderingCtx {
  pub(crate) frame_index: u64,
  pub(crate) ndc: ViewerNDC,
  pub(super) enable_indirect_occlusion_culling: bool,
  pub(super) using_host_driven_indirect_draw: bool,
  pub(super) current_renderer_impl_ty: RasterizationRenderBackendType,
  pub(super) rtx_renderer_enabled: bool,
  pub(super) any_render_change: ChangeNotifier,
  pub(super) lighting: LightSystem,
  pool: AttachmentPool,
  pub(super) gpu: GPU,
  pub swap_chain: ApplicationWindowSurface,
  pub statistics: FramePassStatistics,
  pub enable_statistic_collect: bool,
  pub(super) prefer_bindless_for_indirect_texture_system: bool,

  pub(super) stat_frame_time_in_ms: StatisticStore<f32>,
  pub(super) last_render_timestamp: Option<Instant>,

  pub views: FastHashMap<u64, Viewer3dViewportRenderingCtx>,

  pub(crate) init_config: ViewerInitConfig,
}

impl Viewer3dRenderingCtx {
  pub fn setup_init_config(&self, init_config: &mut ViewerInitConfig) {
    init_config.raster_backend_type = self.current_renderer_impl_ty;
    init_config.enable_indirect_occlusion_culling = self.enable_indirect_occlusion_culling;
    init_config.prefer_bindless_for_indirect_texture_system =
      self.prefer_bindless_for_indirect_texture_system;
    init_config.init_only = self.init_config.init_only.clone();
    init_config.present_mode = self.swap_chain.internal(|v| v.config.present_mode);

    let first_view = self.views.values().next().unwrap();
    init_config.transparent_config = first_view.transparent_config;
    init_config.enable_on_demand_rendering = first_view.enable_on_demand_rendering;
  }

  pub fn gpu(&self) -> &GPU {
    &self.gpu
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
      current_renderer_impl_ty: init_config.raster_backend_type,
      rtx_renderer_enabled: false,
      any_render_change: Default::default(),
      lighting: LightSystem::new(&gpu),
      pool: init_attachment_pool(&gpu),
      statistics: FramePassStatistics::new(64, &gpu),
      gpu,
      views: FastHashMap::default(),
      stat_frame_time_in_ms: StatisticStore::new(200),
      last_render_timestamp: Default::default(),
      init_config: init_config.clone(),
    }
  }

  fn use_viewer_scene_renderer(
    &mut self,
    cx: &mut QueryGPUHookCx,
  ) -> Option<ViewerRendererInstancePreparer> {
    let (cx, change_scope) = cx.use_begin_change_set_collect();

    let camera = use_camera_uniforms(cx, self.ndc);
    let background = use_background(cx);
    let init_config = &self.init_config.init_only;

    let ty = get_suitable_texture_system_ty(
      cx.gpu,
      matches!(
        self.current_renderer_impl_ty,
        RasterizationRenderBackendType::Indirect
      ) || self.rtx_renderer_enabled,
      self.prefer_bindless_for_indirect_texture_system,
    );
    let texture_sys = use_texture_system(cx, ty, &init_config.texture_pool_source_init_config);

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

    let (cx, model_error_state) = cx.use_plain_state_default::<SceneModelErrorRecorder>();

    let raster_scene_renderer = match self.current_renderer_impl_ty {
      RasterizationRenderBackendType::Gles => cx.scope(|cx| {
        let wide_line_renderer_gles = use_widen_line_gles_renderer(cx);

        let mesh =
          use_attribute_mesh_renderer(cx, attributes_custom_key).map(|v| Box::new(v) as Box<_>);

        let unlit_mat = use_unlit_material_uniforms(cx);
        let pbr_mr_mat = use_pbr_mr_material_uniforms(cx);
        let pbr_sg_mat = use_pbr_sg_material_uniforms(cx);

        let materials = cx.when_render(|| {
          Box::new(vec![
            Box::new(unlit_mat.unwrap()) as Box<dyn GLESModelMaterialRenderImpl>,
            Box::new(pbr_mr_mat.unwrap()),
            Box::new(pbr_sg_mat.unwrap()),
          ]) as Box<dyn GLESModelMaterialRenderImpl>
        });

        let std_model = std_model_renderer(cx, materials, mesh);

        let model_renderer = cx.when_render(|| {
          Box::new(vec![
            Box::new(std_model.unwrap()) as Box<dyn GLESModelRenderImpl>,
            Box::new(wide_line_renderer_gles.unwrap()),
          ]) as Box<dyn GLESModelRenderImpl>
        });

        let scene_model_renderer = use_gles_scene_model_renderer(cx, model_renderer);
        cx.when_render(|| GLESSceneRenderer {
          texture_system: texture_sys.clone().unwrap(),
          reversed_depth: self.ndc.enable_reverse_z,
          scene_model_renderer: scene_model_renderer.unwrap(),
          model_error_state: model_error_state.clone(),
        })
        .map(|r| Box::new(r) as Box<dyn SceneRenderer>)
      }),
      RasterizationRenderBackendType::Indirect => cx.scope(|cx| {
        let (cx, change_scope) = cx.use_begin_change_set_collect();

        let enable_combine = init_config.enable_indirect_storage_combine;

        let scope = use_readonly_storage_buffer_combine(cx, "indirect materials", enable_combine);
        let unlit_material = use_unlit_material_storage(cx);
        let pbr_mr_material = use_pbr_mr_material_storage(cx);
        let pbr_sg_material = use_pbr_sg_material_storage(cx);
        scope.end(cx);

        let scope = use_readonly_storage_buffer_combine(cx, "indirect mesh", enable_combine);

        let mesh = use_bindless_mesh(
          cx,
          &init_config.bindless_mesh_init,
          init_config.using_texture_as_storage_buffer_for_indirect_rendering,
          self.init_config.using_host_driven_indirect_draw,
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
        let wide_line = use_widen_line_indirect_renderer(cx, self.using_host_driven_indirect_draw);

        let model_support = cx.when_render(|| {
          Box::new(vec![
            Box::new(std_model.unwrap()) as Box<dyn IndirectModelRenderImpl>,
            Box::new(wide_line.unwrap()),
          ]) as Box<dyn IndirectModelRenderImpl>
        });

        let scene_model = use_indirect_scene_model(cx, model_support);

        let renderer = cx
          .when_render(|| IndirectSceneRenderer {
            texture_system: t_clone.unwrap(),
            renderer: scene_model.map(|v| Box::new(v) as Box<_>).unwrap(),
            reversed_depth: self.ndc.enable_reverse_z,
            using_host_driven_indirect_draw: self.using_host_driven_indirect_draw,
            model_error_state: model_error_state.clone(),
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
            let mesh = use_bindless_mesh(cx, &init_config.bindless_mesh_init, false, false);
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

        let request_reset_sample = any_base_resource_changed.unwrap_or(false)
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

    let extractor = use_default_scene_batch_extractor(cx);

    let camera_transforms = cx
      .use_shared_dual_query_view(GlobalCameraTransformShare(self.ndc))
      .use_assure_result(cx);

    let sm_world_bounding = cx
      .use_shared_dual_query_view(SceneModelWorldBounding)
      .use_assure_result(cx);

    cx.when_render(|| ViewerRendererInstancePreparer {
      camera: camera.unwrap(),
      background: background.unwrap(),
      extractor: extractor.unwrap(),
      raster_scene_renderer: raster_scene_renderer.unwrap(),
      rtx_system: rtx_scene_renderer,
      reversed_depth: self.ndc.enable_reverse_z,
      lighting: lighting.unwrap(),
      culling: culling.unwrap(),
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

  fn storage_allocator(&self) -> Box<dyn AbstractStorageAllocator> {
    if self
      .init_config
      .init_only
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
      waker: futures::task::waker(self.any_render_change.clone()),
      stage: GPUQueryHookStage::Update {
        spawner: task_spawner,
        task_pool: &mut pool,
        change_collector: &mut Default::default(),
      },
      shared_ctx,
      storage_allocator: self.storage_allocator(),
    }
    .execute(|cx| self.use_viewer_scene_renderer(cx));
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
      waker: futures::task::waker(self.any_render_change.clone()),
    }
    .execute(|cx| self.use_viewer_scene_renderer(cx));
  }

  pub fn init_frame(&mut self) {
    self.frame_index += 1;
    let now = Instant::now();
    if let Some(last_frame_time) = self.last_render_timestamp.take() {
      self.stat_frame_time_in_ms.insert(
        now.duration_since(last_frame_time).as_secs_f32() * 1000.,
        self.frame_index,
      );
    }
    self.last_render_timestamp = Some(now);
    self.pool.tick();
  }

  /// return should render views
  pub fn check_should_render_and_copy_cached(
    &mut self,
    target: &RenderTargetView,
    views: &[ViewerViewPort],
  ) -> FastHashSet<(u64, usize)> {
    let upstream = futures::task::noop_waker();
    let any_changed = self.any_render_change.update(&upstream);

    let statistics = self
      .enable_statistic_collect
      .then(|| self.statistics.create_resolver(self.frame_index));

    let mut ctx = FrameCtx::new(&self.gpu, target.size(), &self.pool, statistics);

    let mut new_views = FastHashMap::default();
    for v in views {
      if let Some(view) = self.views.remove(&v.id) {
        new_views.insert(v.id, view);
      } else {
        new_views.insert(
          v.id,
          Viewer3dViewportRenderingCtx::new(&self.gpu, &self.init_config),
        );
      }
    }
    self.views = new_views;

    views
      .iter()
      .enumerate()
      .filter_map(|(i, v)| {
        let view_renderer = self.views.get_mut(&v.id).unwrap();
        if view_renderer.check_should_render_and_copy_cached(target, v, any_changed, &mut ctx) {
          Some((v.id, i))
        } else {
          None
        }
      })
      .collect()
  }

  #[instrument(name = "frame rendering", skip_all)]
  pub fn render(
    &mut self,
    requested_render_views: &FastHashSet<(u64, usize)>,
    final_target: &RenderTargetView,
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
      waker: futures::task::waker(self.any_render_change.clone()),
      storage_allocator: self.storage_allocator(),
    }
    .execute(|cx| self.use_viewer_scene_renderer(cx).unwrap());

    let statistics = self
      .enable_statistic_collect
      .then(|| self.statistics.create_resolver(self.frame_index));

    let mut ctx = FrameCtx::new(&self.gpu, final_target.size(), &self.pool, statistics);

    let lighting_cx = self.lighting.prepare(
      renderer.lighting,
      &mut ctx,
      self.ndc.enable_reverse_z,
      renderer.raster_scene_renderer.as_ref(),
      &renderer.extractor,
      content.scene,
    );

    let renderer = ViewerRendererInstance {
      camera: renderer.camera,
      background: renderer.background,
      raster_scene_renderer: renderer.raster_scene_renderer,
      extractor: renderer.extractor,
      rtx_system: renderer.rtx_system,
      culling: renderer.culling,
      mesh_lod_graph_renderer: renderer.mesh_lod_graph_renderer,
      camera_transforms: renderer.camera_transforms,
      sm_world_bounding: renderer.sm_world_bounding,
      reversed_depth: renderer.reversed_depth,
      lighting: lighting_cx,
    };

    let waker = futures::task::waker(self.any_render_change.clone());
    for (viewport_id, idx) in requested_render_views {
      let view_renderer = self.views.get_mut(viewport_id).unwrap();
      view_renderer.render(&mut ctx, &renderer, content, *idx, final_target, &waker);
    }

    drop(ctx);

    noop_ctx!(cx);
    self.statistics.poll(cx);
  }
}

struct ViewerRendererInstancePreparer {
  pub camera: CameraRenderer,
  pub background: SceneBackgroundRenderer,
  pub raster_scene_renderer: Box<dyn SceneRenderer>,
  pub extractor: DefaultSceneBatchExtractor,
  pub rtx_system: Option<(RayTracingRendererGroup, RtxSystemCore)>,
  pub lighting: LightingRenderingCxPrepareCtx,
  pub culling: ViewerCulling,
  pub mesh_lod_graph_renderer: Option<MeshLODGraphSceneRenderer>,
  pub camera_transforms: BoxedDynQuery<EntityHandle<SceneCameraEntity>, CameraTransform>,
  pub sm_world_bounding: BoxedDynQuery<EntityHandle<SceneModelEntity>, Box3<f64>>,
  pub reversed_depth: bool,
}

pub struct ViewerRendererInstance<'a> {
  pub camera: CameraRenderer,
  pub background: SceneBackgroundRenderer,
  pub raster_scene_renderer: Box<dyn SceneRenderer>,
  pub extractor: DefaultSceneBatchExtractor,
  pub rtx_system: Option<(RayTracingRendererGroup, RtxSystemCore)>,
  pub culling: ViewerCulling,
  pub mesh_lod_graph_renderer: Option<MeshLODGraphSceneRenderer>,
  pub camera_transforms: BoxedDynQuery<EntityHandle<SceneCameraEntity>, CameraTransform>,
  pub sm_world_bounding: BoxedDynQuery<EntityHandle<SceneModelEntity>, Box3<f64>>,
  pub reversed_depth: bool,
  pub lighting: LightingRenderingCx<'a>,
}
