use crate::*;

pub struct PickSceneBlocked;

pub struct PickScene {
  pub enable_hit_debug_log: bool,
}

impl Widget for PickScene {
  fn update_state(&mut self, cx: &mut DynCx) {
    let blocked = cx.message.take::<PickSceneBlocked>().is_some();

    access_cx!(cx, input, PlatformEventInput);
    access_cx!(cx, viewer_scene, Viewer3dSceneCtx);
    access_cx!(cx, picker, Interaction3dCtx);

    if input.state_delta.is_left_mouse_pressing() && !blocked {
      access_cx!(cx, derive, Viewer3dSceneDerive);
      let sms = &derive.sm_to_s;
      let mut main_scene_models = sms.access_multi(&viewer_scene.scene).unwrap();
      let hit = picker
        .picker
        .pick_models_nearest(&mut main_scene_models, picker.mouse_world_ray);
      drop(main_scene_models);

      if let Some(hit) = hit {
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
