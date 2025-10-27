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

  let picker = use_viewer_scene_model_picker(viewer_cx);
  let reader = use_scene_reader(viewer_cx);
  let world_mat = use_global_node_world_mat_view(viewer_cx).use_assure_result(viewer_cx);

  match &mut viewer_cx.stage {
    ViewerCxStage::EventHandling { .. } => {
      let picker = picker.unwrap();
      let reader = reader.unwrap();
      let world_mat = world_mat.expect_resolve_stage().mark_entity_type();

      let widget_env = create_widget_cx(
        &reader,
        &viewer_cx.viewer.scene,
        &picker,
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

      cx.execute(f)
    }
    ViewerCxStage::SceneContentUpdate { writer, .. } => {
      let mut cx = UI3dCx::new_update_stage(
        &mut memory.memory,
        viewer_cx.dyn_cx,
        writer,
        &mut memory.pick_group,
      );

      let mut scene_old = None;
      cx.execute(|cx| {
        cx.on_update(|w, _| {
          scene_old = w.replace_target_scene(widget_scene).into();
        });

        f(cx);

        cx.on_update(|w, _| {
          if let Some(scene) = scene_old.take() {
            w.scene = scene
          }
        });
      });
    }
    _ => {}
  };
}

pub fn create_widget_cx(
  scene_reader: &SceneReader,
  viewer_scene: &Viewer3dContent,
  picker: &ViewerSceneModelPicker,
  world_mat: BoxedDynQuery<EntityHandle<SceneNodeEntity>, Mat4<f64>>,
) -> Box<dyn WidgetEnvAccess> {
  // todo, this is wrong
  let first = viewer_scene.viewports.first().unwrap();
  Box::new(WidgetEnvAccessImpl {
    world_mat,
    camera_node: first.camera_node,
    camera_proj: scene_reader
      .camera
      .read::<SceneCameraPerspective>(first.camera)
      .unwrap(),
    view_logic_pixel_size: picker.camera_view_size_in_logic_pixel.into_u32().into(),
    camera_world_ray: picker.current_mouse_ray_in_world(),
    normalized_canvas_position: picker.normalized_position_ndc(),
  }) as Box<dyn WidgetEnvAccess>
}

struct WidgetEnvAccessImpl {
  world_mat: BoxedDynQuery<EntityHandle<SceneNodeEntity>, Mat4<f64>>,
  camera_node: EntityHandle<SceneNodeEntity>,
  camera_proj: PerspectiveProjection<f32>,
  view_logic_pixel_size: Vec2<u32>,
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

  fn get_view_logic_pixel_size(&self) -> Vec2<u32> {
    self.view_logic_pixel_size
  }
}
