use crate::*;

pub struct Viewer {
  pub surfaces_content: FastHashMap<u32, ViewerSurfaceContent>,
  pub viewport_map: ViewportsImmediate,
  pub rendering_root: RenderingRoot,
  pub rendering: Viewer3dRenderingCtx,
  pub terminal: Terminal,
  pub started_time: Instant,
  pub memory: FunctionMemory,
  pub shared_ctx: SharedHooksCtx,
  pub features_config: ViewerFeaturesInitConfig,
  pub enable_inspection: bool,
  pub use_scene_bvh: bool,
  pub font_system: Arc<RwLock<FontSystem>>,
}

impl Viewer {
  pub fn update_view_ty_immediate(&mut self) {
    // todo, active view
    let mut viewports_map: FastHashMap<_, _> = Default::default();
    for surface in self.surfaces_content.values() {
      for vp in &surface.viewports {
        viewports_map.insert(vp.id, (vp.camera.into_raw(), vp.viewport.zw()));
      }
    }
    self.viewport_map = Arc::new(viewports_map);
  }
}

pub struct ViewerDropCx<'a> {
  pub dyn_cx: &'a mut DynCx,
  pub writer: SceneWriter,
  pub terminal: &'a mut Terminal,
  pub shared_ctx: &'a mut SharedHooksCtx,
  pub inspector: &'a mut Option<&'a mut dyn Inspector>,
}

impl CanCleanUpFrom<ViewerDropCx<'_>> for SharedConsumerToken {
  fn drop_from_cx(&mut self, cx: &mut ViewerDropCx<'_>) {
    if let Some(mem) = cx.shared_ctx.drop_consumer(*self, cx.inspector) {
      mem.write().memory.cleanup_assume_only_plain_states();
    }
  }
}
impl<T> CanCleanUpFrom<ViewerDropCx<'_>> for NothingToDrop<T> {
  fn drop_from_cx(&mut self, _: &mut ViewerDropCx) {}
}

impl CanCleanUpFrom<ViewerDropCx<'_>> for EntityHandle<SceneEntity> {
  fn drop_from_cx(&mut self, cx: &mut ViewerDropCx<'_>) {
    cx.writer.scene_writer.delete_entity(*self);
  }
}

pub fn drop_viewer_from_dyn_cx(viewer: &mut Viewer, dyn_cx: &mut DynCx) {
  let writer = SceneWriter::from_global_some(None);

  let mut dcx = ViewerDropCx {
    dyn_cx,
    writer,
    terminal: &mut viewer.terminal,
    shared_ctx: &mut viewer.shared_ctx,
    inspector: &mut None,
  };
  viewer.memory.cleanup(&mut dcx as *mut _ as *mut ());

  viewer.rendering_root.cleanup(&mut viewer.shared_ctx);

  log::info!("drop viewer from dyn_cx");
}

impl Viewer {
  pub fn new(gpu: GPU, init_config: &ViewerInitConfig, worker: TaskSpawner) -> Self {
    let mut terminal = Terminal::new(worker);
    register_default_commands(&mut terminal);

    let viewer_ndc = ViewerNDC {
      enable_reverse_z: init_config.init_only.enable_reverse_z,
    };

    let font_system = Arc::new(RwLock::new(FontSystem::new()));

    Self {
      surfaces_content: Default::default(),
      viewport_map: Default::default(),
      terminal,
      rendering_root: RenderingRoot::new(&gpu),
      rendering: Viewer3dRenderingCtx::new(gpu, viewer_ndc, init_config, font_system.clone()),
      started_time: Instant::now(),
      memory: Default::default(),
      shared_ctx: Default::default(),
      features_config: init_config.features.clone(),
      enable_inspection: false,
      font_system,
      use_scene_bvh: init_config.use_scene_bvh,
    }
  }

  pub fn drop_surface(&mut self, surface_id: u32) {
    self.surfaces_content.remove(&surface_id);
    self
      .rendering_root
      .drop_surface_render_process_memory(surface_id, &mut self.shared_ctx);
    self.rendering.surface_views.remove(&surface_id);
  }

  pub fn draw_canvas(
    &mut self,
    surface_id: u32,
    canvas: &RenderTargetView,
    task_spawner: &TaskSpawner,
    data_scheduler: &mut ViewerDataScheduler,
    dyn_cx: &mut DynCx,
    inspector: Option<&mut dyn Inspector>,
  ) {
    if let Some(surface_content) = self.surfaces_content.get(&surface_id) {
      self.rendering_root.draw_canvas(
        canvas,
        task_spawner,
        surface_content,
        surface_id,
        &mut self.shared_ctx,
        &mut self.rendering,
        data_scheduler,
        dyn_cx,
        inspector,
        &self.viewport_map,
      );
    } else {
      log::error!("surface {surface_id}'s content not found");
    }
  }

  pub fn ndc(&self) -> &ViewerNDC {
    self.rendering.ndc()
  }

  // todo, currently we only export the swapchain config in exporting window
  pub fn export_init_config(&self, surface: &SurfaceWrapper) -> ViewerInitConfig {
    let mut config = ViewerInitConfig::default();
    self.rendering.setup_init_config(&mut config);

    config.present_mode = surface.internal(|v| v.config.present_mode);
    config.use_scene_bvh = self.use_scene_bvh;

    config.features = self.features_config.clone();
    config
  }
}
