mod rotation;
mod translation;

use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_gui_3d::*;
use rendiation_mesh_generator::*;
use rendiation_scene_core::*;
use rotation::*;
use translation::*;

pub struct GizmoInControl;
pub struct GizmoOutControl;

#[allow(unused_variables)]
fn debug_print(msg: &str) {
  // println!("{}", msg);
}

/// the user should provide Option::<GizmoControlTargetState> for target selecting,
/// and should apply change GizmoUpdateTargetLocal to source object, the applied change should sync
/// back to GizmoControlTargetState
///
/// expect `Option<GizmoControlTargetState>` in ctx
pub fn use_gizmo(cx: &mut UI3dCx) {
  use_inject_cx::<GlobalUIStyle>(cx, |cx| {
    use_inject_cx::<Option<DragStartState>>(cx, |cx| {
      use_group(cx, |cx, root| {
        let auto_scale = ViewAutoScalable {
          independent_scale_factor: 50.,
        };

        cx.on_event(|cx, _, dcx| {
          access_cx!(dcx, start_states, Option::<DragStartState>);
          if start_states.is_some() && cx.platform_event.state_delta.mouse_position_change {
            let action = DragTargetAction {
              camera_world: cx.widget_env.get_camera_world_mat(),
              camera_projection: cx.widget_env.get_camera_proj_mat(),
              world_ray: cx.widget_env.get_camera_world_ray(),
              normalized_screen_position: cx.widget_env.get_normalized_canvas_position(),
            };
            dcx.message.put(action);
            debug_print("dragging");
          }

          if cx.platform_event.state_delta.is_left_mouse_releasing() {
            access_cx_mut!(dcx, start_states, Option::<DragStartState>);
            debug_print("stop drag");
            *start_states = None;
            dcx.message.put(GizmoOutControl);
          }
        });

        use_view_dependent_root(cx, &root, auto_scale, |cx| {
          use_translation_gizmo(cx);
          use_rotation_gizmo(cx);
        });

        cx.on_update(|w, cx| {
          access_cx!(cx, target, Option::<GizmoControlTargetState>);
          let visible = target.is_some();
          let mat = target
            .map(|v| v.target_world_mat)
            .unwrap_or(Mat4::identity());

          w.node_writer
            .write::<SceneNodeVisibleComponent>(root, visible);
          let (t, r, _s) = mat.decompose();
          let mat_with_out_scale = Mat4::translate(t) * Mat4::from(r);
          // assuming our parent world mat is identity

          w.node_writer
            .write::<SceneNodeLocalMatrixComponent>(root, mat_with_out_scale);
        });
      });
    });
  });
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

fn use_axis_interactive_model(
  cx: &mut UI3dCx,
  axis: AxisType,
  mat_init: impl FnOnce(&AxisType) -> Mat4<f32> + 'static,
) {
  let (cx, node) = cx.use_node_entity();
  cx.on_mounting(|w, _, parent| {
    w.node_writer
      .write::<SceneNodeParentIdx>(*node, parent.map(|v| v.into_raw()));
  });
  use_view_independent_node(cx, node, move || mat_init(&axis));

  let (cx, material) = cx.use_unlit_material_entity(|| UnlitMaterialDataView {
    color: Vec4::new(1., 1., 1., 1.),
    color_alpha_tex: None,
    alpha: Default::default(),
  });
  let (cx, model) = cx.use_state_init(|cx| {
    access_cx!(cx.cx, mesh, AttributesMeshEntities);
    UIWidgetModelProxy::new(cx.writer, node, material, mesh)
  });

  use_pickable_model(cx, model);

  if let Some(res) = use_interactive_ui_widget_model(cx, model) {
    if res.mouse_entering {
      access_cx_mut!(cx.dyn_cx, item_state, ItemState);
      item_state.hovering = true;
    }
    if res.mouse_leave {
      access_cx_mut!(cx.dyn_cx, item_state, ItemState);
      item_state.hovering = false;
    }
    if let Some(point) = res.mouse_down {
      access_cx_mut!(cx.dyn_cx, item_state, ItemState);
      item_state.active = true;

      access_cx!(cx.dyn_cx, target, Option::<GizmoControlTargetState>);
      if let Some(target) = target {
        let drag_start_info = target.start_drag(point.position);
        access_cx_mut!(cx.dyn_cx, drag_start, Option::<DragStartState>);
        debug_print("start drag");
        *drag_start = Some(drag_start_info);
        cx.dyn_cx.message.put(GizmoInControl);
      }
    };
  };

  cx.on_update(|w, dcx| {
    access_cx!(dcx, style, GlobalUIStyle);
    access_cx!(dcx, axis_state, AxisActiveState);
    access_cx!(dcx, item_state, ItemState);

    let color = style.get_axis_primary_color(axis);
    let color = map_color(color, *item_state);
    let self_active = item_state.active;
    let visible = !axis_state.has_any_active() || self_active;
    let color = map_color(color, *item_state);

    w.unlit_mat_writer
      .write::<UnlitMaterialColorComponent>(*material, color.expand_with_one());
    w.node_writer
      .write::<SceneNodeVisibleComponent>(*node, visible);
  });
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

#[derive(Debug, Copy, Clone)]
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
  /// x, y: -1 to 1
  normalized_screen_position: Vec2<f32>,
}

pub fn axis_lens(axis: AxisType) -> impl Fn(&mut AxisActiveState) -> &mut ItemState {
  move |s| match axis {
    AxisType::X => &mut s.x,
    AxisType::Y => &mut s.y,
    AxisType::Z => &mut s.z,
  }
}
