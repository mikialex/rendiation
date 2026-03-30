use crate::*;

pub struct Viewer {
  pub content: Viewer3dContent,
  pub surfaces_content: FastHashMap<u32, ViewerSurfaceContent>,
  pub rendering_root: RenderingRoot,
  pub rendering: Viewer3dRenderingCtx,
  pub terminal: Terminal,
  pub background: ViewerBackgroundState,
  pub started_time: Instant,
  pub memory: FunctionMemory,
  pub shared_ctx: SharedHooksCtx,
  pub features_config: ViewerFeaturesInitConfig,
  pub enable_inspection: bool,
  pub font_system: Arc<RwLock<FontSystem>>,
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
  let writer = SceneWriter::from_global(viewer.content.scene);

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
  pub fn new(
    gpu: GPU,
    init_config: &ViewerInitConfig,
    worker: TaskSpawner,
    load_example_cube_tex: impl FnOnce(&mut SceneWriter) -> EntityHandle<SceneTextureCubeEntity>,
  ) -> Self {
    let mut terminal = Terminal::new(worker);
    register_default_commands(&mut terminal);

    let widget_scene = global_entity_of::<SceneEntity>()
      .entity_writer()
      .new_entity(|w| w);

    let root = global_entity_of::<SceneNodeEntity>()
      .entity_writer()
      .new_entity(|w| w);

    let scene = global_entity_of::<SceneEntity>()
      .entity_writer()
      .new_entity(|w| w);

    let scene = Viewer3dContent {
      scene,
      root,
      selected_dir_light: None,
      selected_model: None,
      selected_point_light: None,
      selected_spot_light: None,
      widget_scene,
    };

    let background = {
      let mut writer = SceneWriter::from_global(scene.scene);

      let default_env_background = load_example_cube_tex(&mut writer);
      ViewerBackgroundState::init(default_env_background, &mut writer)
    };

    let viewer_ndc = ViewerNDC {
      enable_reverse_z: init_config.init_only.enable_reverse_z,
    };

    let font_system = Arc::new(RwLock::new(FontSystem::new()));

    Self {
      content: scene,
      surfaces_content: Default::default(),
      terminal,
      rendering_root: RenderingRoot::new(&gpu),
      rendering: Viewer3dRenderingCtx::new(gpu, viewer_ndc, init_config, font_system.clone()),
      background,
      started_time: Instant::now(),
      memory: Default::default(),
      shared_ctx: Default::default(),
      features_config: init_config.features.clone(),
      enable_inspection: false,
      font_system,
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
        &self.content,
        surface_content,
        surface_id,
        &mut self.shared_ctx,
        &mut self.rendering,
        data_scheduler,
        dyn_cx,
        inspector,
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

    config.features = self.features_config.clone();
    config
  }
}
