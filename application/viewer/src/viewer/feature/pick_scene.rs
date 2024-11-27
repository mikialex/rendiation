use crate::*;

pub struct PickScene {
  pub enable_hit_debug_log: bool,
}

impl Widget for PickScene {
  fn update_state(&mut self, cx: &mut DynCx) {
    access_cx!(cx, input, PlatformEventInput);
    access_cx!(cx, viewer_scene, Viewer3dSceneCtx);
    access_cx!(cx, picker, Interaction3dCtx);

    if input.previous_frame_window_state.is_left_mouse_pressed() {
      let sms = global_rev_ref().update_and_read::<SceneModelBelongsToScene>();
      let mut main_scene_models = sms.access_multi(&viewer_scene.scene).unwrap();
      if let Some(hit) = picker
        .picker
        .pick_models_nearest(&mut main_scene_models, picker.mouse_world_ray)
      {
        if self.enable_hit_debug_log {
          dbg!(hit);
        }

        access_cx_mut!(cx, viewer_scene, Viewer3dSceneCtx);
        viewer_scene.selected_target = hit.1.into();
      } else {
        access_cx_mut!(cx, viewer_scene, Viewer3dSceneCtx);
        viewer_scene.selected_target = None;
      }
    }
  }

  fn update_view(&mut self, _cx: &mut DynCx) {}

  fn clean_up(&mut self, _cx: &mut DynCx) {}
}
