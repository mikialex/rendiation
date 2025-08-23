use std::time::Instant;

use crate::*;

mod feature;
pub use feature::*;

mod default_scene;
pub use default_scene::*;

mod pick;
pub use pick::*;

mod terminal;
pub use terminal::*;

mod background;
pub use background::*;

mod widget_bridge;
pub use widget_bridge::*;

mod test_content;
pub use test_content::*;

mod rendering;
pub use rendering::*;

pub const UP: Vec3<f64> = Vec3::new(0., 1., 0.);

pub struct ViewerCx<'a> {
  pub viewer: &'a mut Viewer,
  pub dyn_cx: &'a mut DynCx,

  pub input: &'a PlatformEventInput,
  pub absolute_seconds_from_start: f32,
  pub time_delta_seconds: f32,
  pub task_spawner: &'a TaskSpawner,
  pub change_collector: ChangeCollector,
  stage: ViewerCxStage<'a>,
}

pub struct ViewerDropCx<'a> {
  pub dyn_cx: &'a mut DynCx,
  pub writer: SceneWriter,
  pub terminal: &'a mut Terminal,
  pub shared_ctx: &'a mut SharedHooksCtx,
}

pub struct ViewerInitCx<'a> {
  pub dyn_cx: &'a mut DynCx,
  pub scene: &'a Viewer3dSceneCtx,
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
    let can_flush = matches!(self.stage, ViewerCxStage::Gui { .. });

    let mut drop_cx = if can_flush {
      let writer = SceneWriter::from_global(self.viewer.scene.scene);
      ViewerDropCx {
        dyn_cx: self.dyn_cx,
        writer,
        terminal: &mut self.viewer.terminal,
        shared_ctx: &mut self.viewer.shared_ctx,
      }
      .into()
    } else {
      None
    };

    let drop_cx = drop_cx.as_mut().map(|v| v as *mut _ as *mut ());

    self.viewer.memory.flush(drop_cx)
  }

  fn use_plain_state<T: 'static>(&mut self, f: impl FnOnce() -> T) -> (&mut Self, &mut T) {
    self.use_plain_state_init(|_| f())
  }
}

impl<'a> QueryHookCxLike for ViewerCx<'a> {
  fn is_spawning_stage(&self) -> bool {
    matches!(&self.stage, ViewerCxStage::SpawnTask { .. })
  }

  fn is_resolve_stage(&self) -> bool {
    matches!(&self.stage, ViewerCxStage::EventHandling { .. })
  }

  fn stage(&mut self) -> QueryHookStage {
    match &mut self.stage {
      ViewerCxStage::SpawnTask { pool, .. } => QueryHookStage::SpawnTask {
        spawner: self.task_spawner,
        change_collector: &mut self.change_collector,
        pool,
      },
      ViewerCxStage::EventHandling { task, .. } => QueryHookStage::ResolveTask { task },
      _ => QueryHookStage::Other,
    }
  }

  fn use_shared_consumer(&mut self, key: ShareKey) -> u32 {
    let (_, tk) = self.use_state_init(|fcx| {
      let id = fcx.shared_ctx.next_consumer_id();
      ShaderConsumerToken(id, key)
    });

    tk.0
  }

  fn shared_hook_ctx(&mut self) -> &mut SharedHooksCtx {
    &mut self.viewer.shared_ctx
  }
}
impl<'a> DBHookCxLike for ViewerCx<'a> {}

impl CanCleanUpFrom<ViewerDropCx<'_>> for ShaderConsumerToken {
  fn drop_from_cx(&mut self, cx: &mut ViewerDropCx<'_>) {
    if let Some(mem) = cx.shared_ctx.drop_consumer(self.1, self.0) {
      mem.write().memory.cleanup(cx as *mut _ as *mut ());
    }
    // this check is necessary because not all key need reconcile change
    if let Some(reconciler) = cx.shared_ctx.reconciler.get_mut(&self.1) {
      if reconciler.remove_consumer(self.0) {
        cx.shared_ctx.reconciler.remove(&self.1);
      }
    }
  }
}

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
          scene: &self.viewer.scene,
          terminal: &mut self.viewer.terminal,
          shared_ctx: &mut self.viewer.shared_ctx,
        })
      },
      |state: &mut T, dcx: &mut ViewerDropCx| unsafe {
        state.drop_from_cx(dcx);
        core::ptr::drop_in_place(state);
      },
    );

    (s, state)
  }
}

impl<T> CanCleanUpFrom<ViewerDropCx<'_>> for NothingToDrop<T> {
  fn drop_from_cx(&mut self, _: &mut ViewerDropCx) {}
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
  SceneContentUpdate { writer: &'a mut SceneWriter },
  /// this stage is standalone but not merged with SceneContentUpdate because
  /// user may read write scene freely
  #[non_exhaustive]
  Gui {
    egui_ctx: &'a mut egui::Context,
    global: &'a mut FeaturesGlobalUIStates,
  },
}

pub struct FeaturesGlobalUIStates {
  features: fast_hash_collection::FastHashMap<&'static str, bool>,
}

/// if the function's logic contains cyclic dependency of the outside, using this to
/// make sure the update is synced. for example depend on world matrix while update the local
/// matrix simultaneously.
///
/// todo, improve
pub fn stage_of_update_twice(cx: &mut ViewerCx, internal: impl Fn(&mut ViewerCx)) {
  if cx.is_dynamic_stage() {
    stage_of_update_internal(cx, &internal, false);
    return;
  }
  stage_of_update_internal(cx, &internal, true);
  stage_of_update_internal(cx, &internal, false);
}

/// Act as the viewer event update pair provider.
/// if the different update logic(read write) has dependency, they can be separate by this function.
pub fn stage_of_update(cx: &mut ViewerCx, internal: impl Fn(&mut ViewerCx)) {
  stage_of_update_internal(cx, internal, false);
}

pub fn stage_of_update_internal(
  cx: &mut ViewerCx,
  internal: impl Fn(&mut ViewerCx),
  rollback: bool,
) {
  if let ViewerCxStage::BaseStage = cx.stage {
    let mut pool = AsyncTaskPool::default();
    {
      cx.viewer.shared_ctx.reset_visiting();
      cx.stage = unsafe {
        std::mem::transmute(ViewerCxStage::SpawnTask {
          pool: &mut pool,
          shared_ctx: &mut cx.viewer.shared_ctx,
        })
      };

      cx.execute(&internal, true);
    }

    {
      let mut task_pool_result = pollster::block_on(pool.all_async_task_done());

      cx.viewer.shared_ctx.reset_visiting();
      cx.stage = unsafe {
        std::mem::transmute(ViewerCxStage::EventHandling {
          task: &mut task_pool_result,
          shared_ctx: &mut cx.viewer.shared_ctx,
          terminal_request: cx.viewer.terminal.ctx.store.clone(),
        })
      };

      cx.execute(&internal, true);
    }

    let mut writer = SceneWriter::from_global(cx.viewer.scene.scene);

    cx.stage = unsafe {
      std::mem::transmute(ViewerCxStage::SceneContentUpdate {
        writer: &mut writer,
      })
    };
    cx.execute(&internal, rollback);

    cx.stage = ViewerCxStage::BaseStage;
  } else {
    cx.execute(&internal, rollback);
  }
}

pub fn use_viewer<'a>(
  acx: &'a mut ApplicationCx,
  egui_ctx: &mut egui::Context,
  f: impl Fn(&mut ViewerCx),
) -> &'a mut Viewer {
  let (acx, viewer) = acx.use_plain_state(|| {
    Viewer::new(
      acx.gpu_and_surface.gpu.clone(),
      acx.gpu_and_surface.surface.clone(),
    )
  });

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

  let (acx, worker_thread_pool) =
    acx.use_plain_state(|| TaskSpawner::new("viewer_task_worker", None));

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
    },
  }
  .execute(|viewer| f(viewer), true);

  ViewerCx {
    viewer,
    dyn_cx: acx.dyn_cx,
    input: acx.input,
    absolute_seconds_from_start,
    time_delta_seconds: *frame_time_delta_in_seconds,
    stage: ViewerCxStage::BaseStage,
    task_spawner: worker_thread_pool,
    change_collector: Default::default(),
  }
  .execute(|viewer| f(viewer), true);

  viewer.draw_canvas(&acx.draw_target_canvas, worker_thread_pool);

  viewer
}

pub struct Viewer {
  scene: Viewer3dSceneCtx,
  rendering: Viewer3dRenderingCtx,
  terminal: Terminal,
  background: ViewerBackgroundState,
  started_time: Instant,
  memory: FunctionMemory,
  render_memory: FunctionMemory,
  shared_ctx: SharedHooksCtx,
}

impl CanCleanUpFrom<ApplicationDropCx> for Viewer {
  fn drop_from_cx(&mut self, cx: &mut ApplicationDropCx) {
    let writer = SceneWriter::from_global(self.scene.scene);

    let mut dcx = ViewerDropCx {
      dyn_cx: cx,
      writer,
      terminal: &mut self.terminal,
      shared_ctx: &mut self.shared_ctx,
    };
    self.memory.cleanup(&mut dcx as *mut _ as *mut ());

    todo!();
    // todo, cleanup
    // self
    //   .render_memory
    //   .cleanup(&mut self.render_resource as *mut _ as *mut ());
  }
}

impl Viewer {
  pub fn new(gpu: GPU, swap_chain: ApplicationWindowSurface) -> Self {
    let mut terminal = Terminal::default();
    register_default_commands(&mut terminal);

    let scene = global_entity_of::<SceneEntity>()
      .entity_writer()
      .new_entity();

    let widget_scene = global_entity_of::<SceneEntity>()
      .entity_writer()
      .new_entity();

    let root = global_entity_of::<SceneNodeEntity>()
      .entity_writer()
      .new_entity();

    let camera_node = global_entity_of::<SceneNodeEntity>()
      .entity_writer()
      .with_component_value_writer::<SceneNodeLocalMatrixComponent>(Mat4::lookat(
        Vec3::new(3., 3., 3.),
        Vec3::new(0., 0., 0.),
        Vec3::new(0., 1., 0.),
      ))
      .new_entity();

    let main_camera = global_entity_of::<SceneCameraEntity>()
      .entity_writer()
      .with_component_value_writer::<SceneCameraPerspective>(Some(PerspectiveProjection::default()))
      .with_component_value_writer::<SceneCameraBelongsToScene>(Some(scene.into_raw()))
      .with_component_value_writer::<SceneCameraNode>(Some(camera_node.into_raw()))
      .new_entity();

    let scene = Viewer3dSceneCtx {
      main_camera,
      camera_node,
      scene,
      root,
      selected_target: None,
      widget_scene,
    };

    let background = {
      let mut writer = SceneWriter::from_global(scene.scene);
      load_default_scene(&mut writer, &scene);

      ViewerBackgroundState::init(&mut writer)
    };

    let viewer_ndc = ViewerNDC {
      enable_reverse_z: true,
    };

    Self {
      scene,
      terminal,
      rendering: Viewer3dRenderingCtx::new(gpu, swap_chain, viewer_ndc),
      background,
      started_time: Instant::now(),
      memory: Default::default(),
      render_memory: Default::default(),
      shared_ctx: Default::default(),
    }
  }

  pub fn draw_canvas(&mut self, canvas: &RenderTargetView, task_spawner: &TaskSpawner) {
    let tasks =
      self
        .rendering
        .update_registry(&mut self.render_memory, task_spawner, &mut self.shared_ctx);

    let task_pool_result = pollster::block_on(tasks.all_async_task_done());

    self.rendering.render(
      canvas,
      &self.scene,
      &mut self.render_memory,
      task_pool_result,
      &mut self.shared_ctx,
    );

    self.rendering.tick_frame();
  }
}

pub struct Viewer3dSceneCtx {
  pub main_camera: EntityHandle<SceneCameraEntity>,
  pub camera_node: EntityHandle<SceneNodeEntity>,
  pub root: EntityHandle<SceneNodeEntity>,
  pub scene: EntityHandle<SceneEntity>,
  pub selected_target: Option<EntityHandle<SceneModelEntity>>,
  pub widget_scene: EntityHandle<SceneEntity>,
}

// todo share
fn use_scene_reader(cx: &mut ViewerCx) -> Option<SceneReader> {
  use_scene_reader_internal(cx, cx.viewer.scene.scene)
}

fn use_scene_reader_internal(
  cx: &mut impl DBHookCxLike,
  scene_id: EntityHandle<SceneEntity>,
) -> Option<SceneReader> {
  let mesh_ref_vertex = cx
    .use_db_rev_ref::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>()
    .use_assure_result(cx);

  let node_children = cx
    .use_shared_compute(GlobalNodeConnectivity)
    .use_assure_result(cx);

  let scene_ref_models = cx
    .use_db_rev_ref::<SceneModelBelongsToScene>()
    .use_assure_result(cx);

  cx.when_resolve_stage(|| {
    SceneReader::new_from_global(
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
    )
  })
}
