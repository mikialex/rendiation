use crate::*;

pub struct Viewer {
  pub content: Viewer3dContent,
  pub rendering_root: RenderingRoot,
  pub rendering: Viewer3dRenderingCtx,
  pub terminal: Terminal,
  pub background: ViewerBackgroundState,
  pub started_time: Instant,
  pub memory: FunctionMemory,
  pub shared_ctx: SharedHooksCtx,
  pub features_config: ViewerFeaturesInitConfig,
  pub enable_inspection: bool,
}

impl Viewer {
  pub fn new(
    gpu: GPU,
    swap_chain: SurfaceWrapper,
    init_config: &ViewerInitConfig,
    worker: TaskSpawner,
    load_example_cube_tex: impl FnOnce(&mut SceneWriter) -> EntityHandle<SceneTextureCubeEntity>,
  ) -> Self {
    let mut terminal = Terminal::new(worker);
    register_default_commands(&mut terminal);

    let scene = global_entity_of::<SceneEntity>()
      .entity_writer()
      .new_entity(|w| w);

    let widget_scene = global_entity_of::<SceneEntity>()
      .entity_writer()
      .new_entity(|w| w);

    let root = global_entity_of::<SceneNodeEntity>()
      .entity_writer()
      .new_entity(|w| w);

    let camera_node = global_entity_of::<SceneNodeEntity>()
      .entity_writer()
      .new_entity(|w| {
        w.write::<SceneNodeLocalMatrixComponent>(&Mat4::lookat(
          Vec3::new(3., 3., 3.),
          Vec3::new(0., 0., 0.),
          Vec3::new(0., 1., 0.),
        ))
      });

    let main_camera = global_entity_of::<SceneCameraEntity>()
      .entity_writer()
      .new_entity(|w| {
        w.write::<SceneCameraPerspective>(&Some(PerspectiveProjection::default()))
          .write::<SceneCameraBelongsToScene>(&scene.some_handle())
          .write::<SceneCameraNode>(&camera_node.some_handle())
      });

    let viewport = ViewerViewPort {
      id: alloc_global_res_id(),
      viewport: Default::default(),
      camera: main_camera,
      camera_node,
      debug_camera_for_view_related: None,
    };

    let scene = Viewer3dContent {
      viewports: vec![viewport],
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

    Self {
      content: scene,
      terminal,
      rendering_root: RenderingRoot::new(&gpu, swap_chain),
      rendering: Viewer3dRenderingCtx::new(gpu, viewer_ndc, init_config),
      background,
      started_time: Instant::now(),
      memory: Default::default(),
      shared_ctx: Default::default(),
      features_config: init_config.features.clone(),
      enable_inspection: false,
    }
  }

  pub fn draw_canvas(
    &mut self,
    canvas: &RenderTargetView,
    task_spawner: &TaskSpawner,
    data_scheduler: &mut ViewerDataScheduler,
    dyn_cx: &mut DynCx,
    inspector: Option<&mut dyn Inspector>,
  ) {
    self.rendering_root.draw_canvas(
      canvas,
      task_spawner,
      &self.content,
      &mut self.shared_ctx,
      &mut self.rendering,
      data_scheduler,
      dyn_cx,
      inspector,
    );
  }

  pub fn ndc(&self) -> &ViewerNDC {
    self.rendering.ndc()
  }

  pub fn export_init_config(&self) -> ViewerInitConfig {
    let mut config = ViewerInitConfig::default();
    self.rendering.setup_init_config(&mut config);
    self.rendering_root.setup_init_config(&mut config);
    config.features = self.features_config.clone();
    config
  }
}
