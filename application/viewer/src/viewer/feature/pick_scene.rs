use futures::FutureExt;

use crate::*;

pub struct PickSceneBlocked;

pub struct PickScene {
  pub enable_hit_debug_log: bool,
  pub use_gpu_pick: bool,
  pub gpu_pick_future: Option<Box<dyn Future<Output = Option<u32>> + Unpin>>,
}

impl PickScene {
  fn pick_impl_cpu(
    &mut self,
    derive: &Viewer3dSceneDerive,
    scene: EntityHandle<SceneEntity>,
    picker: &Interaction3dCtx,
  ) -> Option<EntityHandle<SceneModelEntity>> {
    let sms = &derive.sm_to_s;
    let mut main_scene_models = sms.access_multi(&scene).unwrap();
    let hit = picker
      .picker
      .pick_models_nearest(&mut main_scene_models, picker.mouse_world_ray);
    drop(main_scene_models);

    if let Some(hit) = hit {
      if self.enable_hit_debug_log {
        dbg!(hit);
      }
      hit.1.into()
    } else {
      None
    }
  }
}

impl Widget for PickScene {
  fn update_state(&mut self, cx: &mut DynCx) {
    if let Some(f) = &mut self.gpu_pick_future {
      let waker = futures::task::noop_waker_ref();
      let mut ctx = Context::from_waker(waker);
      let ctx = &mut ctx;

      if let Poll::Ready(r) = f.poll_unpin(ctx) {
        println!("gpu pick resolved {:?}", r);
        if let Some(hit_entity_idx) = r {
          // skip the background
          if hit_entity_idx != u32::MAX {
            let hit = global_entity_of::<SceneModelEntity>()
              .entity_reader()
              .reconstruct_handle_by_idx(hit_entity_idx as usize);

            access_cx_mut!(cx, viewer_scene, Viewer3dSceneCtx);
            viewer_scene.selected_target = hit;
          }
        }

        self.gpu_pick_future = None;
      }
    }

    if cx.message.take::<PickSceneBlocked>().is_some() {
      return;
    }
    access_cx!(cx, input, PlatformEventInput);
    if !input.state_delta.is_left_mouse_pressing() {
      return;
    }
    access_cx!(cx, viewer_scene, Viewer3dSceneCtx);
    let scene = viewer_scene.scene;

    access_cx!(cx, picker, Interaction3dCtx);
    let normalized_mouse_position = picker.normalized_mouse_position;

    let mut hit = None;
    if self.use_gpu_pick && self.gpu_pick_future.is_none() {
      access_cx_mut!(cx, renderer, Viewer3dRenderingCtx);
      if let Some(render_size) = renderer.picker.last_id_buffer_size() {
        let point = normalized_mouse_position * Vec2::from(render_size.into_f32());
        let point = point.map(|v| v.floor() as usize);
        if let Some(f) = renderer.picker.pick_point_at((point.x, point.y)) {
          self.gpu_pick_future = Some(f);
        }
      }
    } else {
      access_cx!(cx, picker, Interaction3dCtx);
      access_cx!(cx, derive, Viewer3dSceneDerive);
      hit = self.pick_impl_cpu(derive, scene, picker);
    }

    access_cx_mut!(cx, viewer_scene, Viewer3dSceneCtx);
    viewer_scene.selected_target = hit;
  }

  fn update_view(&mut self, _cx: &mut DynCx) {}
  fn clean_up(&mut self, _cx: &mut DynCx) {}
}
