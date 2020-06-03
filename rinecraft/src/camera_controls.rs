use crate::rinecraft::RinecraftState;
use rendiation_math::Vec2;
use rendiation_render_entity::*;
use rendiation_scenegraph::Index;
use rendium::{EventType, WindowEventSession};

pub enum CameraControllerType {
  FPS,
  ORBIT,
}

pub struct CameraController {
  fps: FPSController,
  orbit: OrbitController,
  active_type: CameraControllerType,
  listener_records: Vec<Index>,
}

impl CameraController {
  pub fn new() -> Self {
    Self {
      fps: FPSController::new(),
      orbit: OrbitController::new(),
      active_type: CameraControllerType::ORBIT, // todo maybe option?
      listener_records: Vec::new(),
    }
  }

  pub fn update(&mut self, camera: &mut impl Camera) -> bool {
    match self.active_type {
      CameraControllerType::FPS => self.fps.update(camera),
      CameraControllerType::ORBIT => self.orbit.update(camera),
    }
  }

  fn remove_all_listeners(&mut self, events: &mut WindowEventSession<RinecraftState>) {
    self
      .listener_records
      .drain(..)
      .map(|i| events.remove_listener(i));
  }

  // todo how can we decouple state path to controller?
  fn attach_orbit(&mut self, events: &mut WindowEventSession<RinecraftState>) {
    self
      .listener_records
      .push(events.add_listener(EventType::MouseMotion, |event_ctx| {
        let state = &mut event_ctx.state;
        if state.window_state.is_left_mouse_down {
          state.camera_controller.orbit.rotate(Vec2::new(
            -state.window_state.mouse_motion.0,
            -state.window_state.mouse_motion.1,
          ))
        }
        if state.window_state.is_right_mouse_down {
          state.camera_controller.orbit.pan(Vec2::new(
            -state.window_state.mouse_motion.0,
            -state.window_state.mouse_motion.1,
          ))
        }
      }));
    self
      .listener_records
      .push(events.add_listener(EventType::MouseWheel, |event_ctx| {
        let state = &mut event_ctx.state;
        let delta = state.window_state.mouse_wheel_delta.1;
        state.camera_controller.orbit.zoom(1.0 - delta * 0.1);
      }))
  }

  fn attach_fps(&mut self, events: &mut WindowEventSession<RinecraftState>) {
    use rendium::winit::event::*;
    self
      .listener_records
      .push(events.add_listener(EventType::MouseMotion, |event_ctx| {
        let state = &mut event_ctx.state;
        state.camera_controller.fps.rotate(Vec2::new(
          state.window_state.mouse_motion.0,
          state.window_state.mouse_motion.1,
        ))
      }));
    self.listener_records.push(events.add_listener_raw(|ctx| {
      let app_state = &mut ctx.state;
      match ctx.event {
        Event::WindowEvent { event, .. } => match event {
          WindowEvent::KeyboardInput {
            input:
              KeyboardInput {
                virtual_keycode: Some(virtual_keycode),
                state,
                ..
              },
            ..
          } => {
            let pressed = *state == ElementState::Pressed;
            if *virtual_keycode == VirtualKeyCode::A {
              app_state.camera_controller.fps.leftward_active = pressed
            } else if *virtual_keycode == VirtualKeyCode::W {
              app_state.camera_controller.fps.forward_active = pressed
            } else if *virtual_keycode == VirtualKeyCode::S {
              app_state.camera_controller.fps.backward_active = pressed
            } else if *virtual_keycode == VirtualKeyCode::D {
              app_state.camera_controller.fps.rightward_active = pressed
            }
          }
          _ => (),
        },
        _ => (),
      }
    }))
  }

  pub fn use_mode(
    &mut self,
    // camera: &impl Camera,
    controller_type: CameraControllerType,
    events: &mut WindowEventSession<RinecraftState>,
  ) -> &mut Self {
    self.remove_all_listeners(events);
    self.active_type = controller_type;
    // todo sync camera state;
    match self.active_type {
      CameraControllerType::FPS => self.attach_fps(events),
      CameraControllerType::ORBIT => self.attach_orbit(events),
    }
    self
  }
}
