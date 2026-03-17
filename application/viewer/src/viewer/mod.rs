use crate::*;

mod feature;
pub use feature::*;

mod default_scene;
pub use default_scene::*;

mod pick;
pub use pick::*;

mod widget_bridge;
pub use widget_bridge::*;

mod test_content;
pub use test_content::*;

pub const UP: Vec3<f64> = Vec3::new(0., 1., 0.);

pub struct ViewerCx<'a> {
  pub viewer: &'a mut Viewer,
  pub dyn_cx: &'a mut DynCx,

  pub input: &'a PlatformEventInput,
  pub absolute_seconds_from_start: f32,
  pub time_delta_seconds: f32,
  pub task_spawner: &'a TaskSpawner,
  pub change_collector: ChangeCollector,
  pub immediate_results: FastHashMap<u32, Arc<dyn Any + Send + Sync>>,
  stage: ViewerCxStage<'a>,
  waker: Waker,
}

impl<'a> ViewerCx<'a> {
  /// this is a workaround for avoid deadlock in use_persistent_db_scope
  pub fn suppress_scene_writer(&mut self) {
    if let ViewerCxStage::SceneContentUpdate { .. } = &self.stage {
      self.stage = ViewerCxStage::SceneContentUpdateSuppressed;
    };
  }

  pub fn re_enable_scene_writer(&mut self) {
    if let ViewerCxStage::SceneContentUpdateSuppressed = &self.stage {
      self.active_scene_writer();
    };
  }

  fn active_scene_writer(&mut self) {
    let writer = SceneWriter::from_global(self.viewer.content.scene);

    self.stage = ViewerCxStage::SceneContentUpdate {
      writer: Box::new(writer),
    };
  }
}

pub struct ViewerInitCx<'a> {
  pub dyn_cx: &'a mut DynCx,
  pub content: &'a Viewer3dContent,
  pub terminal: &'a mut Terminal,
  pub shared_ctx: &'a mut SharedHooksCtx,
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
    if let ViewerCxStage::Gui { inspector, .. } = &mut self.stage {
      let writer = SceneWriter::from_global(self.viewer.content.scene);
      let inspector = unsafe { std::mem::transmute(inspector) };
      let mut drop_cx = ViewerDropCx {
        dyn_cx: self.dyn_cx,
        writer,
        terminal: &mut self.viewer.terminal,
        shared_ctx: &mut self.viewer.shared_ctx,
        inspector,
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
        change_collector: &mut self.change_collector,
        immediate_results: &mut self.immediate_results,
        pool,
      },
      ViewerCxStage::EventHandling { task, .. } => QueryHookStage::ResolveTask { task },
      _ => QueryHookStage::Other,
    }
  }

  fn use_shared_consumer(&mut self, key: ShareKey) -> u32 {
    let (_, tk) = self.use_state_init(|fcx| {
      let id = fcx.shared_ctx.next_consumer_id();
      SharedConsumerToken(id, key)
    });

    tk.0
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
    // this is safe because user can not access previous retrieved state through returned self.
    let s = unsafe { std::mem::transmute_copy(&self) };

    let state = self.viewer.memory.expect_state_init(
      || {
        init(&mut ViewerInitCx {
          dyn_cx: self.dyn_cx,
          content: &self.viewer.content,
          terminal: &mut self.viewer.terminal,
          shared_ctx: &mut self.viewer.shared_ctx,
        })
      },
      |state: &mut T, dcx: &mut ViewerDropCx| {
        state.drop_from_cx(dcx);
      },
    );

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
  f: impl Fn(&mut ViewerCx),
) -> &'a mut Viewer {
  let (acx, worker_thread_pool) = acx.use_plain_state(|| {
    TaskSpawner::new(
      "viewer_task_worker",
      init_config.init_only.thread_pool_thread_count,
    )
  });

  let (acx, data_scheduler) = acx.use_plain_state(ViewerDataScheduler::default);

  let (acx, viewer) = acx.use_state_init(
    || {
      let viewer = Viewer::new(
        acx.gpu_and_surface.gpu.clone(),
        acx.gpu_and_surface.surface.clone(),
        init_config,
        worker_thread_pool.clone(),
        |writer| load_example_cube_tex(writer),
      );
      {
        let mut tex_source = data_scheduler.texture_uri_backend.write();
        let mut mesh_source = data_scheduler.mesh_uri_backend.write();
        let mut writer = SceneWriter::from_global(viewer.content.scene);
        load_default_scene(
          &mut writer,
          &viewer.content,
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

  ViewerCx {
    viewer,
    input: acx.input,
    dyn_cx: acx.dyn_cx,
    absolute_seconds_from_start,
    time_delta_seconds: *frame_time_delta_in_seconds,
    task_spawner: worker_thread_pool,
    change_collector: Default::default(),
    stage: ViewerCxStage::Gui {
      egui_ctx,
      global: gui_feature_global_states,
      inspector: inspection.map(|v| v as &mut dyn Inspector),
    },
    waker: futures::task::noop_waker(),
    immediate_results: Default::default(),
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
    dyn_cx: acx.dyn_cx,
    input: acx.input,
    absolute_seconds_from_start,
    time_delta_seconds: *frame_time_delta_in_seconds,
    stage: ViewerCxStage::BaseStage,
    task_spawner: worker_thread_pool,
    change_collector: Default::default(),
    waker: futures::task::noop_waker(),
    immediate_results: Default::default(),
  }
  .execute(|viewer| f(viewer));

  viewer.draw_canvas(
    &acx.draw_target_canvas,
    worker_thread_pool,
    data_scheduler,
    acx.dyn_cx,
    inspection,
  );

  unsafe {
    acx.dyn_cx.unregister_cx::<ViewerDataScheduler>();
  };

  viewer
}

struct QuerySceneReader(EntityHandle<SceneEntity>);

impl<Cx: DBHookCxLike> SharedResultProvider<Cx> for QuerySceneReader {
  type Result = Arc<SceneReader>;
  fn compute_share_key(&self) -> ShareKey {
    let mut hasher = fast_hash_collection::FastHasher::default();
    std::any::TypeId::of::<Self>().hash(&mut hasher);
    self.0.hash(&mut hasher);
    ShareKey::Hash(std::hash::Hasher::finish(&hasher))
  }

  fn use_logic(&self, cx: &mut Cx) -> UseResult<Self::Result> {
    use_scene_reader_internal(cx, self.0)
  }
}

fn use_scene_reader(cx: &mut ViewerCx) -> Option<Arc<SceneReader>> {
  cx.use_shared_compute(QuerySceneReader(cx.viewer.content.scene))
    .into_resolve_stage()
}

fn use_scene_reader_internal(
  cx: &mut impl DBHookCxLike,
  scene_id: EntityHandle<SceneEntity>,
) -> UseResult<Arc<SceneReader>> {
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
      scene_id,
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
