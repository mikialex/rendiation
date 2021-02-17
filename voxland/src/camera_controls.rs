use rendiation_algebra::Vec2;
use rendiation_render_entity::*;
use winit::event::*;

use crate::{camera::VoxlandCamera, window_event::*, window_states::*};

#[derive(Copy, Clone)]
pub enum CameraControllerType {
  FPS,
  ORBIT,
}

pub struct CameraController<T> {
  fps: FPSController,
  orbit: OrbitController,
  active_type: CameraControllerType,
  listener_records: Vec<WindowEventSessionRemoveToken<T>>,
  self_listeners: Vec<WindowEventSessionRemoveToken<T>>,
}

impl<T> CameraController<T> {
  pub fn new() -> Self {
    Self {
      fps: FPSController::new(),
      orbit: OrbitController::new(),
      active_type: CameraControllerType::ORBIT,
      listener_records: Vec::new(),
      self_listeners: Vec::new(),
    }
  }

  pub fn update(&mut self, camera: &mut VoxlandCamera) -> bool {
    let camera = camera.camera_mut();
    match self.active_type {
      CameraControllerType::FPS => self.fps.update(camera),
      CameraControllerType::ORBIT => self.orbit.update(camera),
    }
  }

  pub fn attach_event<U>(&mut self, events: &mut WindowEventSession<T>, l: U)
  where
    U: FnOnce(&mut T) -> (&mut CameraController<T>, &WindowState) + 'static + Copy,
  {
    let active = &mut events.active;
    use_mode(
      &mut self.listener_records,
      &mut self.active_type,
      CameraControllerType::ORBIT,
      active,
      l,
    );
    // self.self_listeners.push(
    //   EventType::KeyDown.wrap_as_token(active.key_down.on(move |ctx| {
    //     use rendium::winit::event::*;
    //     let virtual_keycode = ctx.event_data;
    //     match virtual_keycode {
    //       VirtualKeyCode::O => {
    //         ctx
    //           .event_update_ctx
    //           .mutate(move |e| use_mode_callback(e, CameraControllerType::ORBIT, l));
    //       }
    //       VirtualKeyCode::F => {
    //         ctx
    //           .event_update_ctx
    //           .mutate(move |e| use_mode_callback(e, CameraControllerType::FPS, l));
    //       }
    //       _ => (),
    //     }
    //   })),
    // );
  }

  fn detach_event(&mut self, events: &mut WindowEventSession<T>) {
    remove_all_active_control_listeners(&mut self.listener_records, &mut events.active);
    self
      .self_listeners
      .drain(..)
      .map(|i| events.active.remove_by_token(i));
  }
}

fn remove_all_active_control_listeners<T>(
  listener_records: &mut Vec<WindowEventSessionRemoveToken<T>>,
  events: &mut WindowEventSessionData<T>,
) {
  listener_records
    .drain(..)
    .map(|i| events.remove_by_token(i));
}

fn use_mode_callback<T, U>(e: &mut EventUpdateCtx<T>, mode: CameraControllerType, l: U)
where
  U: FnOnce(&mut T) -> (&mut CameraController<T>, &WindowState) + 'static + Copy,
{
  let (controller, _) = l(&mut e.state);
  use_mode(
    &mut controller.listener_records,
    &mut controller.active_type,
    CameraControllerType::ORBIT,
    &mut e.active_event,
    l,
  );
}

fn use_mode<T, U>(
  listener_records: &mut Vec<WindowEventSessionRemoveToken<T>>,
  active_type: &mut CameraControllerType,
  controller_type: CameraControllerType,
  events: &mut WindowEventSessionData<T>,
  l: U,
) where
  U: FnOnce(&mut T) -> (&mut CameraController<T>, &WindowState) + 'static + Copy,
{
  remove_all_active_control_listeners(listener_records, events);
  *active_type = controller_type;
  // todo sync camera state;
  match controller_type {
    CameraControllerType::FPS => attach_fps(listener_records, events, l),
    CameraControllerType::ORBIT => attach_orbit(listener_records, events, l),
  }
}

fn attach_orbit<T, U>(
  listener_records: &mut Vec<WindowEventSessionRemoveToken<T>>,
  events: &mut WindowEventSessionData<T>,
  lens: U,
) where
  U: FnOnce(&mut T) -> (&mut CameraController<T>, &WindowState) + 'static + Copy,
{
  listener_records.push(EventType::MouseMotion.wrap_as_token(events.mouse_motion.on(
    move |event_ctx| {
      let state = &mut event_ctx.state;
      let (camera_controller, window_state) = lens(state);
      if window_state.is_left_mouse_down {
        camera_controller.orbit.rotate(Vec2::new(
          -window_state.mouse_motion.0,
          -window_state.mouse_motion.1,
        ))
      }
      if window_state.is_right_mouse_down {
        camera_controller.orbit.pan(Vec2::new(
          -window_state.mouse_motion.0,
          -window_state.mouse_motion.1,
        ))
      }
    },
  )));
  listener_records.push(EventType::MouseWheel.wrap_as_token(events.mouse_wheel.on(
    move |event_ctx| {
      let state = &mut event_ctx.state;
      let (camera_controller, window_state) = lens(state);
      let delta = window_state.mouse_wheel_delta.1;
      camera_controller.orbit.zoom(1.0 - delta * 0.1);
    },
  )))
}

fn attach_fps<T, U>(
  listener_records: &mut Vec<WindowEventSessionRemoveToken<T>>,
  events: &mut WindowEventSessionData<T>,
  lens: U,
) where
  U: FnOnce(&mut T) -> (&mut CameraController<T>, &WindowState) + 'static + Copy,
{
  listener_records.push(EventType::MouseMotion.wrap_as_token(events.mouse_motion.on(
    move |event_ctx| {
      let state = &mut event_ctx.state;
      let (camera_controller, window_state) = lens(state);
      camera_controller.fps.rotate(Vec2::new(
        -window_state.mouse_motion.0,
        window_state.mouse_motion.1,
      ))
    },
  )));
  listener_records.push(
    EventType::KeyInput.wrap_as_token(events.key_input.on(move |ctx| {
      let state = &mut ctx.state;
      let (CameraController { fps, .. }, window_state) = lens(state);
      if let KeyboardInput {
        virtual_keycode: Some(virtual_keycode),
        state,
        ..
      } = ctx.event_data
      {
        let pressed = *state == ElementState::Pressed;
        match virtual_keycode {
          VirtualKeyCode::A => fps.leftward_active = pressed,
          VirtualKeyCode::W => fps.forward_active = pressed,
          VirtualKeyCode::S => fps.backward_active = pressed,
          VirtualKeyCode::D => fps.rightward_active = pressed,
          VirtualKeyCode::Space => fps.ascend_active = pressed,
          VirtualKeyCode::LShift => fps.descend_active = pressed,
          _ => (),
        }
      }
    })),
  )
}
