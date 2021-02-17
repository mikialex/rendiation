use crate::voxland::Voxland;
use rendiation_algebra::*;
use rendiation_render_entity::{raycaster::Raycaster, PerspectiveProjection};

// pub trait WindowEventWatcher<T>{
//   fn init_window(&mut self, session: &mut WindowEventSession<T>);
//   fn un_init_window(&mut self, session: &mut WindowEventSession<T>);
// }

impl Voxland {
  pub fn init_world(&mut self) {
    self.window_session.active.mouse_down.on(|event_ctx| {
      let state = &mut event_ctx.state;
      let x_ratio = state.window_state.mouse_position.0 / state.window_state.size.0;
      let y_ratio = 1. - state.window_state.mouse_position.1 / state.window_state.size.1;
      assert!(x_ratio <= 1.);
      assert!(y_ratio <= 1.);
      let ray = state
        .camera
        .camera()
        .create_screen_ray(Vec2::new(x_ratio, y_ratio));
      state.world.delete_block_by_ray(&ray);
    });
  }
}
