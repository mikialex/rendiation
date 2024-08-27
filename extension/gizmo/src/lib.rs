mod rotation;
mod translation;

use database::*;
use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_gui_3d::*;
use rendiation_mesh_generator::*;
use rendiation_scene_core::*;
pub use rotation::*;
pub use translation::*;

/// the user should provide Option::<GizmoControlTargetState> for target selecting,
/// and should apply change GizmoUpdateTargetLocal to source object, the applied change should sync
/// back to GizmoControlTargetState
pub fn gizmo(v: &mut SceneWriter) -> impl Widget {
  UINode::new(v)
    .with_child(v, translation_gizmo_view)
    .with_child(v, rotation_gizmo_view)
    .with_local_state_inject(Option::<DragStartState>::default())
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
) -> impl FnMut(&mut UIWidgetModel, &mut DynCx) + 'static {
  move |view, cx| {
    access_cx!(cx, style, GlobalUIStyle);
    let color = style.get_axis_primary_color(axis);

    access_cx!(cx, gizmo, AxisActiveState);
    let axis_state = *gizmo.get_axis(axis);
    let self_active = axis_state.active;
    let visible = !gizmo.has_any_active() || self_active;
    let color = map_color(color, axis_state);

    access_cx_mut!(cx, cx3d, SceneWriter);
    view.set_visible(cx3d, visible);
    view.set_color(cx3d, color);
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

fn start_drag(cx: &mut DynCx, pick_position: HitPoint3D) {
  access_cx_mut!(cx, state, ItemState);
  state.active = true;

  access_cx!(cx, target, Option::<GizmoControlTargetState>);
  if let Some(target) = target {
    let drag_start_info = target.start_drag(pick_position.position);
    access_cx_mut!(cx, drag_start, Option::<DragStartState>);
    *drag_start = Some(drag_start_info)
  }
}

fn hovering(cx: &mut DynCx, _: HitPoint3D) {
  access_cx_mut!(cx, state, ItemState);
  state.hovering = true;
}

fn stop_hovering(cx: &mut DynCx) {
  access_cx_mut!(cx, state, ItemState);
  state.hovering = false;
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
pub struct GizmoControlTargetState {
  pub target_local_mat: Mat4<f32>,
  pub target_parent_world_mat: Mat4<f32>,
  pub target_world_mat: Mat4<f32>,
}

pub struct GizmoUpdateTargetLocal(pub Mat4<f32>);

impl GizmoControlTargetState {
  fn start_drag(&self, start_hit_world_position: Vec3<f32>) -> DragStartState {
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
