mod rotation;
mod translation;

use reactive::*;
use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_gui_3d::*;
use rendiation_mesh_generator::*;
use rendiation_scene_core::*;
pub use rotation::*;
pub use translation::*;

pub fn gizmo() -> impl View {
  UINode::default()
    .with_child(translation_gizmo_view)
    .with_child(translation_gizmo_view)
    .with_child(rotation_gizmo_view)
}

#[derive(Copy, Clone, Default, Debug)]
pub struct AxisActiveState {
  pub x: ItemState,
  pub y: ItemState,
  pub z: ItemState,
}

impl AxisActiveState {
  pub fn get_axis(&self, axis: AxisType) -> &ItemState {
    match axis {
      AxisType::X => &self.x,
      AxisType::Y => &self.y,
      AxisType::Z => &self.z,
    }
  }
}

#[derive(Copy, Clone, Default, Debug)]
pub struct ItemState {
  pub hovering: bool,
  pub active: bool,
}

impl AxisActiveState {
  pub fn has_any_active(&self) -> bool {
    self.x.active || self.y.active || self.z.active
  }
  pub fn only_x_active(&self) -> bool {
    self.x.active && !self.y.active && !self.z.active
  }
  pub fn only_y_active(&self) -> bool {
    !self.x.active && self.y.active && !self.z.active
  }
  pub fn only_z_active(&self) -> bool {
    !self.x.active && !self.y.active && self.z.active
  }
  pub fn only_xy_active(&self) -> bool {
    self.x.active && self.y.active && !self.z.active
  }
  pub fn only_yz_active(&self) -> bool {
    !self.x.active && self.y.active && self.z.active
  }
  pub fn only_xz_active(&self) -> bool {
    self.x.active && !self.y.active && self.z.active
  }
}

fn map_color(color: Vec3<f32>, state: ItemState) -> Vec3<f32> {
  if state.hovering && !state.active {
    color + Vec3::splat(0.1)
  } else if state.active {
    color - Vec3::splat(0.1)
  } else {
    color
  }
}

struct StartState {
  start_parent_world_mat: Mat4<f32>,
  start_local_position: Vec3<f32>,
  start_local_quaternion: Quat<f32>,
  start_local_scale: Vec3<f32>,
  start_hit_local_position: Vec3<f32>,
  start_hit_world_position: Vec3<f32>,
}

#[derive(Copy, Clone)]
struct TargetState {
  target_local_mat: Mat4<f32>,
  target_parent_world_mat: Mat4<f32>,
  target_world_mat: Mat4<f32>,
}

#[derive(Clone, Copy)]
struct DragTargetAction {
  camera_world: Mat4<f32>,
  camera_projection: Mat4<f32>,
  world_ray: Ray3<f32>,
  screen_position: Vec2<f32>,
}

#[derive(Clone, Copy)]
pub enum AxisType {
  X,
  Y,
  Z,
}

impl AxisType {
  pub fn dir(&self) -> Vec3<f32> {
    match self {
      AxisType::X => Vec3::new(1., 0., 0.),
      AxisType::Y => Vec3::new(0., 1., 0.),
      AxisType::Z => Vec3::new(0., 0., 1.),
    }
  }
  pub fn mat(&self) -> Mat4<f32> {
    match self {
      AxisType::X => Mat4::rotate_z(-f32::PI() / 2.),
      AxisType::Y => Mat4::identity(),
      AxisType::Z => Mat4::rotate_x(f32::PI() / 2.),
    }
  }
}

pub fn axis_lens(axis: AxisType) -> impl Fn(&mut AxisActiveState) -> &mut ItemState {
  move |s| match axis {
    AxisType::X => &mut s.x,
    AxisType::Y => &mut s.y,
    AxisType::Z => &mut s.z,
  }
}

pub struct GlobalUIStyle {
  pub x_color: Vec3<f32>,
  pub y_color: Vec3<f32>,
  pub z_color: Vec3<f32>,
}

const RED: Vec3<f32> = Vec3::new(0.8, 0.3, 0.3);
const GREEN: Vec3<f32> = Vec3::new(0.3, 0.8, 0.3);
const BLUE: Vec3<f32> = Vec3::new(0.3, 0.3, 0.8);
impl Default for GlobalUIStyle {
  fn default() -> Self {
    Self {
      x_color: RED,
      y_color: GREEN,
      z_color: BLUE,
    }
  }
}

impl GlobalUIStyle {
  pub fn get_axis_primary_color(&self, axis: AxisType) -> Vec3<f32> {
    match axis {
      AxisType::X => self.x_color,
      AxisType::Y => self.y_color,
      AxisType::Z => self.z_color,
    }
  }
}
