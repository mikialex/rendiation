mod gizmo;
use std::{any::Any, marker::PhantomData};

pub use gizmo::*;
use reactive::AllocIdx;
use rendiation_algebra::*;
use rendiation_gui_3d::*;
use rendiation_scene_core::*;

#[derive(Copy, Clone, Default, Debug)]
pub struct AxisActiveState {
  pub x: ItemState,
  pub y: ItemState,
  pub z: ItemState,
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

const RED: Vec3<f32> = Vec3::new(0.8, 0.3, 0.3);
const GREEN: Vec3<f32> = Vec3::new(0.3, 0.8, 0.3);
const BLUE: Vec3<f32> = Vec3::new(0.3, 0.3, 0.8);
fn map_color(color: Vec3<f32>, state: ItemState) -> Vec3<f32> {
  if state.hovering && !state.active {
    color + Vec3::splat(0.1)
  } else if state.active {
    color - Vec3::splat(0.1)
  } else {
    color
  }
}

#[derive(Clone, Copy)]
pub enum AxisType {
  X,
  Y,
  Z,
}

fn arrow_update(
  active: impl Fn(&TranslationGizmo) -> ItemState + 'static,
  axis: AxisType,
) -> impl FnMut(&mut UIModel, &mut ViewStateStore) + 'static {
  let gizmo = StateAccess::<TranslationGizmo>::default();
  let global_style = StateAccess::<GlobalUIStyle>::default();
  move |arrow, model| {
    model.state(&global_style, |style, model| {
      let color = style.get_axis_primary_color(axis);

      model.state(&gizmo, |gizmo, _| {
        let axis_state = active(gizmo);
        let self_active = axis_state.active;
        arrow.set_visible(!gizmo.active_state.has_any_active() || self_active);
        arrow.set_color(map_color(color, axis_state));
      });
    });
  }
}

fn arrow() -> UIModel {
  UIModel::default().with_shape(ArrowShape::default().build())
}

pub struct GlobalUIStyle {
  x_color: Vec3<f32>,
  y_color: Vec3<f32>,
  z_color: Vec3<f32>,
}

impl GlobalUIStyle {
  pub fn get_axis_primary_color(&self, axis: AxisType) -> Vec3<f32> {
    todo!()
  }
}

#[derive(Default)]
struct TranslationGizmo {
  active_state: AxisActiveState,
}

pub fn translation_gizmo_view() -> impl View {
  let x_dir = Mat4::rotate_z(-f32::PI() / 2.);
  let x_arrow = arrow()
    .with_matrix(x_dir)
    .with_view_update(arrow_update(|s| s.active_state.x, AxisType::X))
    .with_on_mouse_down(|m, position| {
      //
    });

  let y_dir = Mat4::identity();
  let y_arrow = arrow().with_matrix(y_dir);

  let z_dir = Mat4::rotate_x(f32::PI() / 2.);
  let z_arrow = arrow().with_matrix(z_dir);

  UIGroup::default()
    .with_child(x_arrow)
    .with_child(y_arrow)
    .with_child(z_arrow)
}
