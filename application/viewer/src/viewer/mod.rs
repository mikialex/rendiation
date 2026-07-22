use crate::*;

mod example;
pub use example::*;

mod feature;
pub use feature::*;

mod widget;
pub use widget::*;

mod default_scene;
pub use default_scene::*;

mod pick;
pub use pick::*;

mod widget_bridge;
pub use widget_bridge::*;

mod test_content;
pub use test_content::*;

mod background;
pub use background::*;

pub const UP: Vec3<f64> = Vec3::new(0., 1., 0.);

pub struct DefaultSceneInfo {
  /// default scene should not be removed, it will contains examples.
  pub scene: EntityHandle<SceneEntity>,
  pub background_state: ViewerBackgroundState,
}

pub struct ViewerCx<'a> {
  pub viewer: &'a mut Viewer,
  pub dyn_cx: &'a mut DynCx,

  pub input: &'a WindowEventStates,
  pub current_window_swapchain: &'a SurfaceWrapper,
  pub surface_id: u32,
  pub active_surface_content: &'a mut ViewerSurfaceContent,
  pub app_features: &'a mut ViewerAppFeaturesConfig,
  pub default_scene: &'a mut DefaultSceneInfo,

  // this id should be immutable
  pub widget_scene: EntityHandle<SceneEntity>,

  pub absolute_seconds_from_start: f32,
  pub time_delta_seconds: f32,
  pub task_spawner: &'a TaskSpawner,
  pub immediate_results: FastHashMap<u32, Arc<dyn Any + Send + Sync>>,
  pub trace_event_notifier: &'a dyn Fn(ViewerTracingEvent),
  stage: ViewerCxStage<'a>,
  waker: Waker,
}

impl<'a> ViewerCx<'a> {
  fn active_scene_writer(&mut self) {
    let writer = SceneWriter::from_global();

    self.stage = ViewerCxStage::SceneContentUpdate {
      writer: Box::new(writer),
    };
  }
}

pub struct ViewerInitCx<'a> {
  pub dyn_cx: &'a mut DynCx,
  pub content: &'a ViewerSurfaceContent,
  pub surface_id: u32,
  pub terminal: &'a mut Terminal,
  pub shared_ctx: &'a mut SharedHooksCtx,
  pub app_features: &'a mut ViewerAppFeaturesConfig,
}

unsafe impl HooksCxLike for ViewerCx<'_> {
  fn memory_mut(&mut self) -> &mut FunctionMemory {
    &mut self.viewer.memory
  }
  fn memory_ref(&self) -> &FunctionMemory {
    &self.viewer.memory
  }
  fn is_dynamic_stage(&self) -> bool {
    matches!(self.stage, ViewerCxStage::Gui { .. })
  }
  fn flush(&mut self) {
    if let ViewerCxStage::Gui { .. } = &mut self.stage {
      let writer = SceneWriter::from_global();
      let mut drop_cx = ViewerDropCx {
        dyn_cx: self.dyn_cx,
        writer,
        terminal: &mut self.viewer.terminal,
      };

      let drop_cx = &mut drop_cx as *mut _ as *mut ();

      self.viewer.memory.flush(drop_cx)
    }
  }

  fn use_plain_state<T: 'static>(&mut self, f: impl FnOnce() -> T) -> (&mut Self, &mut T) {
    self.use_plain_state_init(|_| f())
  }
}

impl InspectableCx for ViewerCx<'_> {
  fn if_inspect(&mut self, f: impl FnOnce(&mut dyn Inspector)) {
    if let ViewerCxStage::Gui {
      inspector: Some(inspector),
      ..
    } = &mut self.stage
    {
      std::hint::cold_path();
      f(*inspector);
    }
  }
}

impl<'a> QueryHookCxLike for ViewerCx<'a> {
  fn is_spawning_stage(&self) -> bool {
    matches!(&self.stage, ViewerCxStage::SpawnTask { .. })
  }

  fn is_resolve_stage(&self) -> bool {
    matches!(&self.stage, ViewerCxStage::EventHandling { .. })
  }

  fn waker(&mut self) -> &mut Waker {
    &mut self.waker
  }

  fn dyn_env(&mut self) -> &mut DynCx {
    self.dyn_cx
  }

  fn stage(&mut self) -> QueryHookStage<'_> {
    match &mut self.stage {
      ViewerCxStage::SpawnTask { pool, .. } => QueryHookStage::SpawnTask {
        spawner: self.task_spawner,
        immediate_results: &mut self.immediate_results,
        pool,
      },
      ViewerCxStage::EventHandling { task, .. } => QueryHookStage::ResolveTask { task },
      _ => QueryHookStage::Other,
    }
  }

  fn use_shared_consumer(&mut self, key: ShareKey, debug_label: &str) -> u32 {
    let (_, tk) = self.use_state_init(|fcx| {
      let id = fcx.shared_ctx.next_consumer_id();
      let dropper = fcx.shared_ctx.create_dropper();
      SharedConsumerToken {
        id,
        key,
        debug_label: debug_label.to_string(),
        dropper: Arc::new(vec![dropper]),
      }
    });

    tk.id
  }

  fn shared_hook_ctx(&mut self) -> &mut SharedHooksCtx {
    &mut self.viewer.shared_ctx
  }
}
impl<'a> DBHookCxLike for ViewerCx<'a> {}

impl<'a> ViewerCx<'a> {
  pub fn use_plain_state<T>(&mut self) -> (&mut Self, &mut T)
  where
    T: Any + Default,
  {
    self.use_plain_state_init(|_| T::default())
  }

  pub fn use_plain_state_init<T>(
    &mut self,
    init: impl FnOnce(&mut ViewerInitCx) -> T,
  ) -> (&mut Self, &mut T)
  where
    T: Any,
  {
    let (cx, s) = self.use_state_init(|cx| NothingToDrop(init(cx)));
    (cx, &mut s.0)
  }

  pub fn use_state_init<T>(
    &mut self,
    init: impl FnOnce(&mut ViewerInitCx) -> T,
  ) -> (&mut Self, &mut T)
  where
    T: Any + for<'x> CanCleanUpFrom<ViewerDropCx<'x>>,
  {
    let this = self as *mut Self;
    let state = unsafe {
      (*this).viewer.memory.expect_state_init(
        || {
          init(&mut ViewerInitCx {
            dyn_cx: self.dyn_cx,
            content: &self.active_surface_content,
            terminal: &mut self.viewer.terminal,
            shared_ctx: &mut self.viewer.shared_ctx,
            surface_id: self.surface_id,
            app_features: &mut self.app_features,
          })
        },
        |state: &mut T, dcx: &mut ViewerDropCx| {
          state.drop_from_cx(dcx);
        },
      )
    };
    let s = unsafe { &mut *this };

    (s, state)
  }
}

#[non_exhaustive]
pub enum ViewerCxStage<'a> {
  #[non_exhaustive]
  BaseStage,
  SpawnTask {
    pool: &'a mut AsyncTaskPool,
    shared_ctx: &'a mut SharedHooksCtx,
  },
  EventHandling {
    task: &'a mut TaskPoolResultCx,
    shared_ctx: &'a mut SharedHooksCtx,
    terminal_request: TerminalTaskStore,
  },
  #[non_exhaustive]
  SceneContentUpdate {
    writer: Box<SceneWriter>,
  },
  SceneContentUpdateSuppressed,
  /// this stage is standalone but not merged with SceneContentUpdate because
  /// user may read write scene freely
  #[non_exhaustive]
  Gui {
    egui_ctx: &'a mut egui::Context,
    global: &'a mut FeaturesGlobalUIStates,
    /// if None, then the inspection is disabled
    inspector: Option<&'a mut dyn Inspector>,
  },
}

pub struct FeaturesGlobalUIStates {
  features: fast_hash_collection::FastHashMap<&'static str, bool>,
}

/// expand the viewer cx base stage to a series of stages, and call them multiple times
/// because some logic may have cyclic dependency for example something depend on world matrix
#[track_caller]
pub fn stage_of_update(cx: &mut ViewerCx, cycle_count: usize, internal: impl Fn(&mut ViewerCx)) {
  cx.raw_scope(|cx| {
    if let ViewerCxStage::Gui { inspector, .. } = &mut cx.stage {
      cx.viewer.shared_ctx.flush_drop_queue(&mut |key| {
        if let Some(ins) = inspector {
          ins.drop_shared_ctx(key);
        }
      });
    }
    if let ViewerCxStage::BaseStage = cx.stage {
      for _ in 0..cycle_count {
        let mut pool = AsyncTaskPool::default();
        {
          cx.viewer.shared_ctx.reset_visiting();
          cx.immediate_results.clear();
          cx.stage = unsafe {
            std::mem::transmute(ViewerCxStage::SpawnTask {
              pool: &mut pool,
              shared_ctx: &mut cx.viewer.shared_ctx,
            })
          };

          cx.execute(&internal);
        }

        {
          let mut task_pool_result = pollster::block_on(pool.all_async_task_done());

          cx.viewer.shared_ctx.reset_visiting();
          task_pool_result
            .token_based_result
            .extend(cx.immediate_results.drain());
          cx.immediate_results.clear();
          cx.stage = unsafe {
            std::mem::transmute(ViewerCxStage::EventHandling {
              task: &mut task_pool_result,
              shared_ctx: &mut cx.viewer.shared_ctx,
              terminal_request: cx.viewer.terminal.ctx.store.clone(),
            })
          };

          cx.execute(&internal);
        }

        cx.active_scene_writer();
        cx.execute(&internal);
      }

      cx.stage = ViewerCxStage::BaseStage;
    } else {
      cx.execute(internal);
    }
  })
}

pub fn use_viewer<'a>(
  acx: &'a mut ApplicationCx,
  egui_ctx: &mut egui::Context,
  init_config: &ViewerInitConfig,
  app_init_config: &ViewerAppFeaturesConfig,
  trace_event_notifier: &dyn Fn(ViewerTracingEvent),
  f: impl Fn(&mut ViewerCx),
) -> &'a mut Viewer {
  let (acx, worker_thread_pool) = acx.use_plain_state(|| {
    TaskSpawner::new(
      "viewer_task_worker",
      init_config.init_only.thread_pool_thread_count,
    )
  });

  let (acx, data_scheduler) = acx.use_plain_state(|| {
    let exe_path = std::env::current_exe().unwrap();
    let root = exe_path.parent().unwrap().join("temp_resources/");
    ViewerDataScheduler::new(Some(&root))
  });

  let (acx, scene_instances) = acx.use_plain_state(|| {
    let scene = global_entity_of::<SceneEntity>()
      .entity_writer()
      .new_entity(|w| w);

    let background = {
      let mut writer = SceneWriter::from_global();

      let default_env_background = load_example_cube_tex(&mut writer);
      ViewerBackgroundState::init(default_env_background, &mut writer, scene)
    };
    DefaultSceneInfo {
      scene,
      background_state: background,
    }
  });

  let surface_id = acx.surface_id;
  let (acx, viewer) = acx.use_state_init(
    || {
      let mut viewer = Viewer::new(
        acx.gpu_and_surface.gpu.clone(),
        init_config,
        worker_thread_pool.clone(),
      );

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
            .write::<SceneCameraNode>(&camera_node.some_handle())
        });

      let viewport = ViewerViewPort {
        id: alloc_global_res_id(),
        viewport: Default::default(),
        camera: main_camera,
        camera_node,
        debug_camera_for_view_related: None,
        scene: scene_instances.scene,
      };

      let surface_content = ViewerSurfaceContent {
        viewports: vec![viewport],
        device_pixel_ratio: 1.0,
      };
      // we construct the default view in our viewer application
      viewer.surfaces_content.insert(surface_id, surface_content);
      // this is necessary, as we unwrap to access surface views in our update cycles
      viewer
        .rendering
        .surface_views
        .entry(surface_id)
        .or_default();

      {
        let mut tex_source = data_scheduler.texture_uri_backend.write();
        let mut mesh_source = data_scheduler.mesh_uri_backend.write();
        let mut writer = SceneWriter::from_global();
        load_default_scene(
          &mut writer,
          scene_instances.scene,
          tex_source.as_mut(),
          mesh_source.as_mut(),
        );
      };

      viewer
    },
    drop_viewer_from_dyn_cx,
  );

  let (acx, gui_feature_global_states) = acx.use_plain_state(|| FeaturesGlobalUIStates {
    features: Default::default(),
  });

  let (acx, widget_scene) = acx.use_plain_state(|| {
    global_entity_of::<SceneEntity>()
      .entity_writer()
      .new_entity(|w| w)
  });

  let (acx, app_features) = acx.use_plain_state(|| app_init_config.clone());

  let (acx, axis) = acx.use_plain_state(|| WorldCoordinateAxis::new(&acx.gpu_and_surface.gpu));

  let (acx, tick_timestamp) = acx.use_plain_state(Instant::now);
  let (acx, frame_time_delta_in_seconds) = acx.use_plain_state(|| 0.0);

  let absolute_seconds_from_start = Instant::now()
    .duration_since(viewer.started_time)
    .as_secs_f32();

  let now = Instant::now();
  *frame_time_delta_in_seconds = now.duration_since(*tick_timestamp).as_secs_f32();
  *tick_timestamp = now;

  let (acx, ins) = acx.use_plain_state(InspectedContent::default);
  let inspection = viewer.enable_inspection.then_some(&mut *ins);

  unsafe {
    acx
      .dyn_cx
      .register_cx::<ViewerDataScheduler>(data_scheduler);
  };

  viewer.update_view_ty_immediate();

  #[cfg(all(feature = "dhat-heap-profiling", not(target_family = "wasm")))]
  let _dhat_profiler = if viewer.should_trace_next_frame_allocation_info {
    viewer.should_trace_next_frame_allocation_info = false;
    log::info!("dhat heap profiling started for this frame");
    Some(dhat::Profiler::builder().trim_backtraces(None).build())
  } else {
    None
  };

  let mut active_surface_content = viewer.surfaces_content.remove(&acx.surface_id).unwrap();
  // always sync
  active_surface_content.device_pixel_ratio = acx.input.window_state.device_pixel_ratio;

  ViewerCx {
    viewer,
    widget_scene: *widget_scene,
    input: acx.input,
    dyn_cx: acx.dyn_cx,
    absolute_seconds_from_start,
    active_surface_content: &mut active_surface_content,
    current_window_swapchain: &acx.gpu_and_surface.surface,
    time_delta_seconds: *frame_time_delta_in_seconds,
    surface_id: acx.surface_id,
    task_spawner: worker_thread_pool,
    stage: ViewerCxStage::Gui {
      egui_ctx,
      global: gui_feature_global_states,
      inspector: inspection.map(|v| v as &mut dyn Inspector),
    },
    waker: futures::task::noop_waker(),
    immediate_results: Default::default(),
    trace_event_notifier,
    app_features,
    default_scene: scene_instances,
  }
  .execute(|viewer| f(viewer));

  if viewer.enable_inspection {
    ins.draw(egui_ctx);
    ins.clear();
  }

  let inspection = viewer
    .enable_inspection
    .then_some(&mut *ins as &mut dyn Inspector);

  ViewerCx {
    viewer,
    widget_scene: *widget_scene,
    dyn_cx: acx.dyn_cx,
    input: acx.input,
    absolute_seconds_from_start,
    active_surface_content: &mut active_surface_content,
    current_window_swapchain: &acx.gpu_and_surface.surface,
    time_delta_seconds: *frame_time_delta_in_seconds,
    stage: ViewerCxStage::BaseStage,
    task_spawner: worker_thread_pool,
    waker: futures::task::noop_waker(),
    immediate_results: Default::default(),
    surface_id: acx.surface_id,
    trace_event_notifier,
    app_features,
    default_scene: scene_instances,
  }
  .execute(|viewer| f(viewer));

  viewer
    .surfaces_content
    .insert(acx.surface_id, active_surface_content);

  trace_event_notifier(ViewerTracingEvent::Render);

  viewer.draw_canvas(
    acx.surface_id,
    &acx.draw_target_canvas,
    worker_thread_pool,
    data_scheduler,
    acx.dyn_cx,
    inspection,
    &mut ViewerAppFrameRenderingExtension {
      widget_scene: *widget_scene,
      axis,
    },
  );

  unsafe {
    acx.dyn_cx.unregister_cx::<ViewerDataScheduler>();
  };

  viewer
}

struct QuerySceneReader;

impl<Cx: DBHookCxLike> SharedResultProvider<Cx> for QuerySceneReader {
  type Result = Arc<SceneReader>;

  share_provider_hash_type_id! {}

  fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
    use_scene_reader_internal(cx)
  }
}

fn use_scene_reader(cx: &mut ViewerCx) -> Option<Arc<SceneReader>> {
  cx.use_shared_compute(QuerySceneReader).into_resolve_stage()
}

fn use_scene_reader_internal(cx: &mut impl DBHookCxLike) -> UseResult<Arc<SceneReader>> {
  let mesh_ref_vertex = cx
    .use_db_rev_ref::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>()
    .use_assure_result(cx);

  let node_children = cx
    .use_shared_compute(GlobalNodeConnectivity)
    .use_assure_result(cx);

  let scene_ref_models = cx
    .use_db_rev_ref::<SceneModelBelongsToScene>()
    .use_assure_result(cx);

  let r = cx.when_resolve_stage(|| {
    let reader = SceneReader::new_from_global(
      mesh_ref_vertex
        .expect_resolve_stage()
        .mark_foreign_key::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>()
        .into_boxed_multi(),
      node_children
        .expect_resolve_stage()
        .mark_entity_type_multi::<SceneNodeEntity>()
        .multi_map(|k| unsafe { EntityHandle::<SceneNodeEntity>::from_raw(k) })
        .into_boxed_multi(),
      scene_ref_models
        .expect_resolve_stage()
        .mark_foreign_key::<SceneModelBelongsToScene>()
        .into_boxed_multi(),
    );
    Arc::new(reader)
  });

  if let Some(r) = r {
    UseResult::ResolveStageReady(r)
  } else {
    UseResult::NotInStage
  }
}
