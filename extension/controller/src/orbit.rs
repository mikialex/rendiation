use rendiation_algebra::Vec2;
use rendiation_algebra::*;
use rendiation_geometry::Spherical;

use crate::*;

pub struct OrbitController {
  pub spherical: Spherical,

  pub rotate_angle_factor: f32,
  pub pan_factor: f32,
  pub zoom_factor: f32,

  // restriction
  pub max_polar_angle: f32,
  pub min_polar_angle: f32,

  pub spherical_delta: Spherical,
  pub zooming: f32,
  pub pan_offset: Vec3<f32>,

  pub view_width: f32,
  pub view_height: f32,
}

impl Default for OrbitController {
  fn default() -> Self {
    Self::new()
  }
}

impl OrbitController {
  pub fn new() -> Self {
    Self {
      spherical: Spherical::new(),

      rotate_angle_factor: 2.,
      pan_factor: 0.02,
      zoom_factor: 0.3,

      // restriction over how down you can look,
      // should strictly less than Pi and greater than [`min_polar_angle`]
      max_polar_angle: std::f32::consts::PI - 0.001,

      // restriction over how up you can look,
      // should strictly greater than 0 and less than [`max_polar_angle`]
      min_polar_angle: 0.001,

      // motion
      spherical_delta: Spherical::new(),
      zooming: 1.0,
      pan_offset: Vec3::new(0.0, 0.0, 0.0),

      view_width: 1000.,
      view_height: 1000.,
    }
  }

  pub fn pan(&mut self, offset: Vec2<f32>) {
    let mut offset = offset.rotate(Vector::zero(), -self.spherical.azim);
    offset *= self.spherical.radius * self.pan_factor;
    self.pan_offset.x += offset.x;
    self.pan_offset.z += offset.y;
  }

  pub fn zoom(&mut self, factor: f32) {
    self.zooming = 1. + (factor - 1.) * self.zoom_factor;
  }

  pub fn rotate(&mut self, offset: Vec2<f32>) {
    self.spherical_delta.polar +=
      offset.y / self.view_height * std::f32::consts::PI * self.rotate_angle_factor;
    self.spherical_delta.azim -=
      offset.x / self.view_width * std::f32::consts::PI * self.rotate_angle_factor;
  }

  fn reset_delta(&mut self) {
    self.spherical_delta.reset_pose();
    self.zooming = 1.0;
    self.pan_offset = Vec3::new(0.0, 0.0, 0.0);
  }
}

impl OrbitController {
  pub fn update_target_and_position(&mut self, target: Vec3<f32>, position: Vec3<f32>) {
    self.spherical = Spherical::from_sphere_point_and_center(target - position, position);
    self.reset_delta()
  }

  pub fn update(&mut self) -> Option<(Vec3<f32>, Vec3<f32>)> {
    if self.spherical_delta.azim.abs() < 0.0001
      && self.spherical_delta.polar.abs() < 0.0001
      && (self.zooming - 1.).abs() < 0.0001
      && self.pan_offset.length2() < 0.000_000_1
    {
      return None;
    }

    self.spherical.radius *= self.zooming;

    self.spherical.azim += self.spherical_delta.azim;

    self.spherical.polar = (self.spherical.polar + self.spherical_delta.polar)
      .clamp(self.min_polar_angle, self.max_polar_angle);

    self.spherical.center += self.pan_offset;

    let eye = self.spherical.to_sphere_point();

    self.reset_delta();

    (eye, self.spherical.center).into()
  }
}

#[derive(Default)]
pub struct OrbitWinitWindowState {
  is_left_mouse_down: bool,
  is_right_mouse_down: bool,
  mouse_position: Vec2<f32>,
}

use winit::event::*;
impl OrbitController {
  pub fn event<T>(
    &mut self,
    s: &mut OrbitWinitWindowState,
    event: &winit::event::Event<T>,
    bound: InputBound,
    pause: bool,
  ) {
    if pause {
      s.is_left_mouse_down = false;
      s.is_right_mouse_down = false;
      return;
    }
    match event {
      Event::WindowEvent { event, .. } => match event {
        WindowEvent::MouseInput { button, state, .. } => {
          if let ElementState::Pressed = state {
            if !bound.is_point_in(s.mouse_position) {
              return;
            }
          }

          match button {
            MouseButton::Left => match state {
              ElementState::Pressed => s.is_left_mouse_down = true,
              ElementState::Released => s.is_left_mouse_down = false,
            },
            MouseButton::Right => match state {
              ElementState::Pressed => s.is_right_mouse_down = true,
              ElementState::Released => s.is_right_mouse_down = false,
            },
            _ => {}
          }
        }
        WindowEvent::CursorMoved { position, .. } => {
          s.mouse_position.x = position.x as f32;
          s.mouse_position.y = position.y as f32;
        }
        WindowEvent::MouseWheel { delta, .. } => match delta {
          MouseScrollDelta::LineDelta(_, y) => {
            self.zoom(1.0 - y * 0.1);
          }
          MouseScrollDelta::PixelDelta(physical_position) => {
            self.zoom(1.0 - physical_position.y as f32 * 0.01);
          }
        },
        WindowEvent::PinchGesture { delta, .. } => {
          self.zoom(1.0 - *delta as f32 * 10.);
        }
        _ => {}
      },
      Event::DeviceEvent { event, .. } => match event {
        DeviceEvent::MouseMotion { delta } => {
          if s.is_left_mouse_down {
            self.rotate(Vec2::new(-delta.0 as f32, -delta.1 as f32))
          }

          if s.is_right_mouse_down {
            self.pan(Vec2::new(-delta.0 as f32, -delta.1 as f32))
          }
        }
        _ => {}
      },
      _ => {}
    }
  }
}
