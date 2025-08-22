use crate::{viewer::use_scene_reader, *};

#[derive(Default)]
struct UI3DMemory {
  memory: FunctionMemory,
  pick_group: WidgetSceneModelIntersectionGroupConfig,
}

impl CanCleanUpFrom<ViewerDropCx<'_>> for UI3DMemory {
  fn drop_from_cx(&mut self, cx: &mut ViewerDropCx) {
    self.memory.cleanup(&mut UI3dBuildCx {
      writer: &mut cx.writer,
      cx: cx.dyn_cx,
      pick_group: &mut self.pick_group,
    } as *mut _ as *mut ());
  }
}

pub fn widget_root(viewer_cx: &mut ViewerCx, f: impl FnOnce(&mut UI3dCx)) {
  let (viewer_cx, memory) = viewer_cx.use_state_init(|_| UI3DMemory::default());
  let widget_scene = viewer_cx.viewer.scene.widget_scene;

  #[allow(unused_assignments)] // false positive?
  let mut interaction = None;

  let picker = use_viewer_picker(viewer_cx);
  let reader = use_scene_reader(viewer_cx);
  let world_mat = use_global_node_world_mat_view(viewer_cx).use_assure_result(viewer_cx);

  match &mut viewer_cx.stage {
    ViewerCxStage::EventHandling { .. } => {
      let picker = picker.unwrap();
      let reader = reader.unwrap();
      let world_mat = world_mat.expect_resolve_stage().mark_entity_type();

      // todo, fix , this should use actual render resolution instead of full window size
      let canvas_resolution = Vec2::new(
        viewer_cx.input.window_state.physical_size.0
          / viewer_cx.input.window_state.device_pixel_ratio,
        viewer_cx.input.window_state.physical_size.1
          / viewer_cx.input.window_state.device_pixel_ratio,
      )
      .map(|v| v.ceil() as u32);

      let widget_env = create_widget_cx(
        &reader,
        &viewer_cx.viewer.scene,
        &picker,
        canvas_resolution,
        world_mat.into_boxed(),
      );
      let picker = &picker;
      let reader = &reader;
      let widget_env = widget_env.as_ref();

      interaction = Some(prepare_picking_state(picker, &memory.pick_group));
      let mut cx = UI3dCx::new_event_stage(
        &mut memory.memory,
        UIEventStageCx {
          platform_event: viewer_cx.input,
          interaction_cx: interaction.as_ref().unwrap(),
          widget_env,
        },
        reader,
        viewer_cx.dyn_cx,
        &mut memory.pick_group,
      );

      if cx.is_creating() {
        // skip the first event stage when first time init
        return;
      }

      cx.execute(f, true)
    }
    ViewerCxStage::SceneContentUpdate { writer, .. } => {
      let mut cx = UI3dCx::new_update_stage(
        &mut memory.memory,
        viewer_cx.dyn_cx,
        writer,
        &mut memory.pick_group,
      );

      let mut scene_old = None;
      cx.execute(
        |cx| {
          cx.on_update(|w, _| {
            scene_old = w.replace_target_scene(widget_scene).into();
          });

          f(cx);

          cx.on_update(|w, _| {
            if let Some(scene) = scene_old.take() {
              w.scene = scene
            }
          });
        },
        true,
      );
    }
    _ => {}
  };
}

pub fn create_widget_cx(
  scene_reader: &SceneReader,
  viewer_scene: &Viewer3dSceneCtx,
  picker: &ViewerPicker,
  canvas_resolution: Vec2<u32>,
  world_mat: BoxedDynQuery<EntityHandle<SceneNodeEntity>, Mat4<f64>>,
) -> Box<dyn WidgetEnvAccess> {
  Box::new(WidgetEnvAccessImpl {
    world_mat,
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
