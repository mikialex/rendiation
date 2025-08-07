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
  stage: ViewerCxStage<'a>,
}

pub struct ViewerDropCx<'a> {
  pub dyn_cx: &'a mut DynCx,
  pub writer: &'a mut SceneWriter,
  pub terminal: &'a mut Terminal,
}

pub struct ViewerInitCx<'a> {
  pub dyn_cx: &'a mut DynCx,
  pub scene: &'a Viewer3dSceneCtx,
  pub derive: &'a Viewer3dSceneDeriveSource,
  pub terminal: &'a mut Terminal,
}

unsafe impl HooksCxLike for ViewerCx<'_> {
  fn memory_mut(&mut self) -> &mut FunctionMemory {
    &mut self.viewer.memory
  }
  fn memory_ref(&self) -> &FunctionMemory {
    &self.viewer.memory
  }
  fn flush(&mut self) {
    self.viewer.memory.flush(self.dyn_cx as *mut _ as *mut ())
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
    #[derive(Default)]
    struct PlainState<T>(T);
    impl<T> CanCleanUpFrom<ViewerDropCx<'_>> for PlainState<T> {
      fn drop_from_cx(&mut self, _: &mut ViewerDropCx) {}
    }

    let (cx, s) = self.use_state_init(|cx| PlainState(init(cx)));
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
          derive: &self.viewer.derives,
          terminal: &mut self.viewer.terminal,
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

#[non_exhaustive]
pub enum ViewerCxStage<'a> {
  #[non_exhaustive]
  BaseStage,
  EventHandling {
    reader: &'a SceneReader,
    picker: &'a ViewerPicker,
    derived: &'a Viewer3dSceneDerive,
    widget_cx: &'a dyn WidgetEnvAccess,
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
    {
      let derived = cx.viewer.derives.poll_update();

      let picker = ViewerPicker::new(&derived, cx.input, cx.viewer.scene.main_camera);

      let scene_reader = SceneReader::new_from_global(
        cx.viewer.scene.scene,
        derived.mesh_vertex_ref.clone(),
        derived.node_children.clone(),
        derived.sm_to_s.clone(),
      );
      // todo, fix , this should use actual render resolution instead of full window size
      let canvas_resolution = Vec2::new(
        cx.input.window_state.physical_size.0 / cx.input.window_state.device_pixel_ratio,
        cx.input.window_state.physical_size.1 / cx.input.window_state.device_pixel_ratio,
      )
      .map(|v| v.ceil() as u32);

      let widget_env = create_widget_cx(
        &derived,
        &scene_reader,
        &cx.viewer.scene,
        &picker,
        canvas_resolution,
      );

      cx.stage = unsafe {
        std::mem::transmute(ViewerCxStage::EventHandling {
          reader: &scene_reader,
          picker: &picker,
          derived: &derived,
          widget_cx: widget_env.as_ref(),
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
  let (acx, viewer) = acx.use_plain_state_init(|| {
    Viewer::new(
      acx.gpu_and_surface.gpu.clone(),
      acx.gpu_and_surface.surface.clone(),
    )
  });

  let (acx, gui_feature_global_states) = acx.use_plain_state_init(|| FeaturesGlobalUIStates {
    features: Default::default(),
  });

  let (acx, tick_timestamp) = acx.use_plain_state_init(Instant::now);
  let (acx, frame_time_delta_in_seconds) = acx.use_plain_state_init(|| 0.0);

  let absolute_seconds_from_start = Instant::now()
    .duration_since(viewer.started_time)
    .as_secs_f32();

  let now = Instant::now();
  *frame_time_delta_in_seconds = now.duration_since(*tick_timestamp).as_secs_f32();
  *tick_timestamp = now;

  ViewerCx {
    viewer,
    dyn_cx: acx.dyn_cx,
    input: acx.input,
    absolute_seconds_from_start,
    time_delta_seconds: *frame_time_delta_in_seconds,
    stage: ViewerCxStage::BaseStage,
  }
  .execute(|viewer| f(viewer), true);

  viewer
    .rendering
    .update_registry(&mut viewer.render_memory, &mut viewer.render_resource);

  let derived = viewer.derives.poll_update();
  viewer.draw_canvas(&acx.draw_target_canvas, &derived);
  viewer.rendering.tick_frame();

  ViewerCx {
    viewer,
    input: acx.input,
    dyn_cx: acx.dyn_cx,
    absolute_seconds_from_start,
    time_delta_seconds: *frame_time_delta_in_seconds,
    stage: ViewerCxStage::Gui {
      egui_ctx,
      global: gui_feature_global_states,
    },
  }
  .execute(|viewer| f(viewer), true);

  viewer
}

pub struct Viewer {
  scene: Viewer3dSceneCtx,
  rendering: Viewer3dRenderingCtx,
  derives: Viewer3dSceneDeriveSource,
  terminal: Terminal,
  background: ViewerBackgroundState,
  started_time: Instant,
  memory: FunctionMemory,
  render_memory: FunctionMemory,
  render_resource: ReactiveQueryCtx,
}

impl CanCleanUpFrom<ApplicationDropCx> for Viewer {
  fn drop_from_cx(&mut self, cx: &mut ApplicationDropCx) {
    let mut writer = SceneWriter::from_global(self.scene.scene);

    let mut dcx = ViewerDropCx {
      dyn_cx: cx,
      writer: &mut writer,
      terminal: &mut self.terminal,
    };
    self.memory.cleanup(&mut dcx as *mut _ as *mut ());
    self
      .render_memory
      .cleanup(&mut self.render_resource as *mut _ as *mut ());
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

    let camera_transforms = camera_transforms(viewer_ndc)
      .into_boxed()
      .into_static_forker();

    let derives = Viewer3dSceneDeriveSource {
      world_mat: scene_node_derive_world_mat().into_boxed().into_forker(),
      node_net_visible: Box::new(scene_node_derive_visible()),
      camera_transforms: camera_transforms.clone(),
      mesh_vertex_ref: Box::new(
        global_rev_ref()
          .watch_inv_ref::<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>(),
      ),
      sm_to_s: Box::new(global_rev_ref().watch_inv_ref::<SceneModelBelongsToScene>()),
      sm_world_bounding: Box::new(scene_model_world_bounding()),
      node_children: Box::new(scene_node_connectivity_many_one_relation()),
    };

    Self {
      scene,
      terminal,
      rendering: Viewer3dRenderingCtx::new(gpu, swap_chain, viewer_ndc, camera_transforms),
      derives,
      background,
      started_time: Instant::now(),
      memory: Default::default(),
      render_memory: Default::default(),
      render_resource: Default::default(),
    }
  }

  pub fn draw_canvas(&mut self, canvas: &RenderTargetView, scene_derive: &Viewer3dSceneDerive) {
    self.rendering.render(
      canvas,
      &self.scene,
      scene_derive,
      &mut self.render_memory,
      &mut self.render_resource,
    )
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

pub struct Viewer3dSceneDeriveSource {
  pub world_mat: RQForker<EntityHandle<SceneNodeEntity>, Mat4<f64>>,
  pub node_net_visible: BoxedDynReactiveQuery<EntityHandle<SceneNodeEntity>, bool>,
  pub camera_transforms: RQForker<EntityHandle<SceneCameraEntity>, CameraTransform>,
  pub mesh_vertex_ref:
    RevRefOfForeignKeyWatch<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>,
  pub sm_to_s: RevRefOfForeignKeyWatch<SceneModelBelongsToScene>,
  pub sm_world_bounding: BoxedDynReactiveQuery<EntityHandle<SceneModelEntity>, Box3<f64>>,
  pub node_children:
    BoxedDynReactiveOneToManyRelation<EntityHandle<SceneNodeEntity>, EntityHandle<SceneNodeEntity>>,
}

impl Viewer3dSceneDeriveSource {
  fn poll_update(&self) -> Viewer3dSceneDerive {
    noop_ctx!(cx);

    let (_, world_mat) = self.world_mat.describe(cx).resolve_kept();
    let (_, node_net_visible) = self.node_net_visible.describe(cx).resolve_kept();
    let (_, camera_transforms) = self.camera_transforms.describe(cx).resolve_kept();
    let (_, mesh_vertex_ref) = self
      .mesh_vertex_ref
      .describe_with_inv_dyn(cx)
      .resolve_kept();
    let (_, sm_to_s) = self.sm_to_s.describe_with_inv_dyn(cx).resolve_kept();
    let (_, sm_world_bounding) = self.sm_world_bounding.describe(cx).resolve_kept();
    let (_, node_children) = self.node_children.describe_with_inv_dyn(cx).resolve_kept();
    Viewer3dSceneDerive {
      world_mat: world_mat.into_boxed(),
      camera_transforms: camera_transforms.into_boxed(),
      mesh_vertex_ref: mesh_vertex_ref.into_boxed_multi(),
      node_net_visible: node_net_visible.into_boxed(),
      sm_world_bounding: sm_world_bounding.into_boxed(),
      node_children: node_children.into_boxed_multi(),
      sm_to_s: sm_to_s.into_boxed_multi(),
    }
  }
}

/// used in render & scene update
#[derive(Clone)]
pub struct Viewer3dSceneDerive {
  pub world_mat: BoxedDynQuery<EntityHandle<SceneNodeEntity>, Mat4<f64>>,
  pub node_net_visible: BoxedDynQuery<EntityHandle<SceneNodeEntity>, bool>,
  pub node_children:
    BoxedDynMultiQuery<EntityHandle<SceneNodeEntity>, EntityHandle<SceneNodeEntity>>,
  pub camera_transforms: BoxedDynQuery<EntityHandle<SceneCameraEntity>, CameraTransform>,
  pub mesh_vertex_ref:
    RevRefOfForeignKey<AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity>,
  pub sm_world_bounding: BoxedDynQuery<EntityHandle<SceneModelEntity>, Box3<f64>>,
  pub sm_to_s: RevRefOfForeignKey<SceneModelBelongsToScene>,
}

pub fn create_widget_cx(
  derived: &Viewer3dSceneDerive,
  scene_reader: &SceneReader,
  viewer_scene: &Viewer3dSceneCtx,
  picker: &ViewerPicker,
  canvas_resolution: Vec2<u32>,
) -> Box<dyn WidgetEnvAccess> {
  Box::new(WidgetEnvAccessImpl {
    world_mat: derived.world_mat.clone(),
    camera_node: viewer_scene.camera_node,
    camera_proj: scene_reader
      .camera
      .read::<SceneCameraPerspective>(viewer_scene.main_camera)
      .unwrap(),
    canvas_resolution,
    camera_world_ray: picker.current_mouse_ray_in_world(),
    normalized_canvas_position: picker.normalized_position_ndc(),
  }) as Box<dyn WidgetEnvAccess>
}

struct WidgetEnvAccessImpl {
  world_mat: BoxedDynQuery<EntityHandle<SceneNodeEntity>, Mat4<f64>>,
  camera_node: EntityHandle<SceneNodeEntity>,
  camera_proj: PerspectiveProjection<f32>,
  canvas_resolution: Vec2<u32>,
  camera_world_ray: Ray3<f64>,
  // xy -1 to 1
  normalized_canvas_position: Vec2<f32>,
}

impl WidgetEnvAccess for WidgetEnvAccessImpl {
  fn get_world_mat(&self, sm: EntityHandle<SceneNodeEntity>) -> Option<Mat4<f64>> {
    self.world_mat.access(&sm)
  }

  fn get_camera_node(&self) -> EntityHandle<SceneNodeEntity> {
    self.camera_node
  }

  fn get_camera_perspective_proj(&self) -> PerspectiveProjection<f32> {
    self.camera_proj
  }

  fn get_camera_world_ray(&self) -> Ray3<f64> {
    self.camera_world_ray
  }

  fn get_normalized_canvas_position(&self) -> Vec2<f32> {
    self.normalized_canvas_position
  }

  fn get_view_resolution(&self) -> Vec2<u32> {
    self.canvas_resolution
  }
}
