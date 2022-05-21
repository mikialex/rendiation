use rendiation_algebra::Vec2;
use rendiation_algebra::*;
use rendiation_geometry::Spherical;

use crate::{Controller, ControllerWinitEventSupport, InputBound, Transformed3DControllee};

pub struct OrbitController {
  pub spherical: Spherical,

  pub rotate_angle_factor: f32,
  pub pan_factor: f32,
  pub zoom_factor: f32,

  // restriction
  pub max_polar_angle: f32,
  pub min_polar_angle: f32,

  // damping
  pub spherical_delta: Spherical,
  pub zooming: f32,
  pub pan_offset: Vec3<f32>,

  pub enable_damping: bool,
  pub zooming_damping_factor: f32,
  pub rotate_damping_factor: f32,
  pub pan_damping_factor: f32,

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

      rotate_angle_factor: 0.2,
      pan_factor: 0.0002,
      zoom_factor: 0.3,

      // restriction
      max_polar_angle: std::f32::consts::PI,
      min_polar_angle: 0.01,

      // damping
      spherical_delta: Spherical::new(),
      zooming: 1.0,
      pan_offset: Vec3::new(0.0, 0.0, 0.0),

      enable_damping: true,
      zooming_damping_factor: 0.1,
      rotate_damping_factor: 0.1,
      pan_damping_factor: 0.1,

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
    self.spherical_delta.azim +=
      offset.x / self.view_width * std::f32::consts::PI * self.rotate_angle_factor;
  }

  fn reset_damping(&mut self) {
    self.spherical_delta.reset_pose();
    self.zooming = 1.0;
    self.pan_offset = Vec3::new(0.0, 0.0, 0.0);
  }
}

impl Controller for OrbitController {
  fn sync(&mut self, target: &dyn Transformed3DControllee) {
    let mat = target.matrix();
    let position_new = *mat * Vec3::new(0., 0., -1.);
    let origin = mat.position();
    let position_dir = position_new - origin;
    self.spherical = Spherical::from_sphere_point_and_center(position_dir, origin);
    self.reset_damping()
  }

  fn update(&mut self, target: &mut dyn Transformed3DControllee) -> bool {
    if self.spherical_delta.azim.abs() < 0.0001
      && self.spherical_delta.polar.abs() < 0.0001
      && (self.zooming - 1.).abs() < 0.0001
      && self.pan_offset.length2() < 0.000_000_1
    {
      return false;
    }

    self.spherical.radius *= self.zooming;

    self.spherical.azim += self.spherical_delta.azim;

    self.spherical.polar = (self.spherical.polar + self.spherical_delta.polar)
      .max(self.min_polar_angle)
      .min(self.max_polar_angle);

    self.spherical.center += self.pan_offset;

    let matrix = target.matrix_mut();
    let eye = self.spherical.to_sphere_point();
    *matrix = Mat4::lookat(eye, self.spherical.center, Vec3::new(0.0, 1.0, 0.0));

    // update damping effect
    if self.enable_damping {
      self.spherical_delta.azim *= 1. - self.rotate_damping_factor;
      self.spherical_delta.polar *= 1. - self.rotate_damping_factor;
      self.zooming += (1. - self.zooming) * self.zooming_damping_factor;
      self.pan_offset *= 1. - self.pan_damping_factor;
    } else {
      self.reset_damping();
    }
    true
  }
}

#[derive(Default)]
pub struct OrbitWinitWindowState {
  is_left_mouse_down: bool,
  is_right_mouse_down: bool,
  mouse_position: Vec2<f32>,
}

use winit::event::*;
impl ControllerWinitEventSupport for OrbitController {
  type State = OrbitWinitWindowState;
  fn event<T>(&mut self, s: &mut Self::State, event: &winit::event::Event<T>, bound: InputBound) {
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
        WindowEvent::MouseWheel { delta, .. } => {
          if let MouseScrollDelta::LineDelta(_, y) = delta {
            self.zoom(1.0 - y * 0.1);
          }
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
