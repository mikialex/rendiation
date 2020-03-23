use crate::rinecraft::Rinecraft;
use rendiation_math::*;
use rendiation_render_entity::raycaster::Raycaster;

impl Rinecraft{
  pub fn use_orbit_controller(&mut self){
    self.window_session.add_mouse_motion_listener(|event_ctx| {
      let state = &mut event_ctx.state;
      if state.window_state.is_left_mouse_down {
        state.orbit_controller.rotate(Vec2::new(
          -state.window_state.mouse_motion.0,
          -state.window_state.mouse_motion.1,
        ))
      }
      if state.window_state.is_right_mouse_down {
        state.orbit_controller.pan(Vec2::new(
          -state.window_state.mouse_motion.0,
          -state.window_state.mouse_motion.1,
        ))
      }
    });
    
    self.window_session.add_mouse_wheel_listener(|event_ctx| {
      let state = &mut event_ctx.state;
      let delta = state.window_state.mouse_wheel_delta.1;
      state.orbit_controller.zoom(1.0 - delta * 0.1);
    });
  }

  pub fn init_world(&mut self){
    self.window_session.add_mouse_down_listener(|event_ctx| {
      let state = &mut event_ctx.state;
      let x_ratio = state.window_state.mouse_position.0 / state.window_state.size.0;
      let y_ratio = 1. - state.window_state.mouse_position.1 / state.window_state.size.1;
      assert!(x_ratio <= 1.);
      assert!(y_ratio <= 1.);
      let ray = state.camera.create_screen_ray(Vec2::new(x_ratio, y_ratio));
      state.world.delete_block_by_ray(&ray);
    });
  }
}

