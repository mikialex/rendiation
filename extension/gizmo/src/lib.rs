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
    .with_child(rotation_gizmo_view)
    .with_local_state_inject(Option::<DragStartState>::default())
    .with_local_state_inject(Option::<GizmoControlTargetState>::default())
    .with_local_state_inject(GlobalUIStyle::default())
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

  pub fn get_rest_axis(&self, axis: AxisType) -> (&ItemState, &ItemState) {
    match axis {
      AxisType::X => (&self.y, &self.z),
      AxisType::Y => (&self.x, &self.z),
      AxisType::Z => (&self.x, &self.y),
    }
  }

  pub fn get_rest_axis_mut(&mut self, axis: AxisType) -> (&mut ItemState, &mut ItemState) {
    match axis {
      AxisType::X => (&mut self.y, &mut self.z),
      AxisType::Y => (&mut self.x, &mut self.z),
      AxisType::Z => (&mut self.x, &mut self.y),
    }
  }
}

pub fn update_per_axis_model(
  axis: AxisType,
) -> impl FnMut(&mut UIWidgetModel, &mut StateStore) + 'static {
  move |view, model| {
    let color = model.state_get::<GlobalUIStyle, _>(|style| style.get_axis_primary_color(axis));

    model.state::<AxisActiveState>(|gizmo| {
      let axis_state = *gizmo.get_axis(axis);
      let self_active = axis_state.active;
      view.set_visible(!gizmo.has_any_active() || self_active);
      view.set_color(map_color(color, axis_state));
    });
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

struct DragStartState {
  start_parent_world_mat: Mat4<f32>,
  start_local_position: Vec3<f32>,
  start_local_quaternion: Quat<f32>,
  start_local_scale: Vec3<f32>,
  start_hit_local_position: Vec3<f32>,
  start_hit_world_position: Vec3<f32>,
}

#[derive(Copy, Clone)]
struct GizmoControlTargetState {
  target_local_mat: Mat4<f32>,
  target_parent_world_mat: Mat4<f32>,
  target_world_mat: Mat4<f32>,
  target_node: AllocIdx<SceneNodeEntity>,
}

impl GizmoControlTargetState {
  pub fn update_target_local_mat(&mut self, target_local: Mat4<f32>) {
    todo!()
  }
  pub fn start_drag(&self, start_hit_world_position: Vec3<f32>) -> DragStartState {
    let (t, r, s) = self.target_local_mat.decompose();
    DragStartState {
      start_parent_world_mat: self.target_parent_world_mat,
      start_local_position: t,
      start_local_quaternion: r,
      start_local_scale: s,
      start_hit_local_position: self.target_world_mat.inverse_or_identity()
        * start_hit_world_position,
      start_hit_world_position,
    }
  }
}

#[derive(Clone, Copy)]
struct DragTargetAction {
  camera_world: Mat4<f32>,
  camera_projection: Mat4<f32>,
  world_ray: Ray3<f32>,
  screen_position: Vec2<f32>,
}

pub fn axis_lens(axis: AxisType) -> impl Fn(&mut AxisActiveState) -> &mut ItemState {
  move |s| match axis {
    AxisType::X => &mut s.x,
    AxisType::Y => &mut s.y,
    AxisType::Z => &mut s.z,
  }
}
