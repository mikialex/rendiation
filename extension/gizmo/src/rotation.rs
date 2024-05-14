use crate::*;

pub fn rotation_gizmo_view(
  parent: AllocIdx<SceneNodeEntity>,
  v: &mut View3dProvider,
) -> impl View3d {
  let mut rotate_state = Option::<RotateState>::default();
  UIGroup::default()
    .with_child(build_rotator(v, AxisType::X, parent))
    .with_child(build_rotator(v, AxisType::Y, parent))
    .with_child(build_rotator(v, AxisType::Z, parent))
    .with_state_post_update(move |cx| {
      if let Some(drag_action) = cx.message.take::<DragTargetAction>() {
        state_access!(cx, target, Option::<GizmoControlTargetState>);
        state_access!(cx, rotate_view, AxisActiveState);
        state_access!(cx, start_states, Option::<DragStartState>);
        let start_states = start_states.as_ref().unwrap();

        if let Some(target) = target {
          if let Some(action) = handle_rotating(
            start_states,
            target,
            &mut rotate_state,
            rotate_view,
            drag_action,
          ) {
            cx.message.put(GizmoUpdateTargetLocal(action))
          }
        }
      }
    })
    .with_local_state_inject(AxisActiveState::default())
    .with_local_state_inject(Option::<RotateState>::default())
}

pub fn build_rotator(
  v: &mut View3dProvider,
  axis: AxisType,
  parent: AllocIdx<SceneNodeEntity>,
) -> impl View3d {
  let mesh = build_attributes_mesh(|builder| {
    builder.triangulate_parametric(
      &TorusMeshParameter {
        radius: 1.5,
        tube_radius: 0.03,
      }
      .make_surface(),
      TessellationConfig { u: 36, v: 4 },
      true,
    );
  });

  let degree_90 = f32::PI() / 2.;
  let mat = match axis {
    AxisType::X => Mat4::rotate_y(degree_90),
    AxisType::Y => Mat4::rotate_x(degree_90),
    AxisType::Z => Mat4::identity(),
  };

  UIWidgetModel::new(v)
    .with_shape(v, mesh)
    .with_parent(v, parent)
    .with_matrix(v, mat)
    .with_on_mouse_down(start_drag)
    .with_on_mouse_hovering(hovering)
    .with_on_mouse_out(stop_hovering)
    .with_view_update(update_per_axis_model(axis))
    .with_state_pick(axis_lens(axis))
}

struct RotateState {
  current_angle_all: f32,
  last_dir: Vec2<f32>,
}

fn handle_rotating(
  states: &DragStartState,
  target: &GizmoControlTargetState,
  rotate_state: &mut Option<RotateState>,
  axis: &AxisActiveState,
  action: DragTargetAction,
) -> Option<Mat4<f32>> {
  #[rustfmt::skip]
  // new_hit_world = M(parent) * M(local_translate) * M(new_local_rotate) * M(local_scale) * start_hit_local_position =>
  // M-1(local_translate) * M-1(parent) * new_hit_world =  M(new_local_rotate) * M(local_scale) * start_hit_local_position
  // should we support world space point align like above? but the question is, we have to also modify scale, because
  // it's maybe impossible to rotate one point to the other if your rotation center is origin.

  // here we use simple screen space rotation match local space to see the effects.

  let vp = action.camera_projection * action.camera_world.inverse()?;

  let start_hit_screen_position = (vp * states.start_hit_world_position).xy();
  let pivot_center_screen_position = (vp * target.target_world_mat.position()).xy();

  let origin_dir = start_hit_screen_position - pivot_center_screen_position;
  let origin_dir = origin_dir.normalize();
  let new_dir = action.screen_position - pivot_center_screen_position;
  let new_dir = new_dir.normalize();

  let RotateState {
    current_angle_all,
    last_dir,
  } = rotate_state.get_or_insert_with(|| RotateState {
    current_angle_all: 0.,
    last_dir: origin_dir,
  });

  let rotate_dir = last_dir.cross(new_dir).signum();
  // min one is preventing float precision issue which will cause nan in acos
  let angle_delta = last_dir.dot(new_dir).min(1.).acos() * rotate_dir;

  *current_angle_all += angle_delta;
  *last_dir = new_dir;

  let axis = if axis.only_x_active() {
    Vec3::new(1., 0., 0.)
  } else if axis.only_y_active() {
    Vec3::new(0., 1., 0.)
  } else if axis.only_z_active() {
    Vec3::new(0., 0., 1.)
  } else {
    return None;
  };

  let camera_world_position = action.camera_world.position();

  let view_dir = camera_world_position - target.target_world_mat.position();

  let axis_world = axis.transform_direction(target.target_world_mat);
  let mut angle = *current_angle_all;
  if axis_world.dot(view_dir) < 0. {
    angle = -angle;
  }

  let quat = Quat::rotation(axis, angle);

  let new_local = Mat4::translate(states.start_local_position)
    * Mat4::from(states.start_local_quaternion)
    * Mat4::from(quat)
    * Mat4::scale(states.start_local_scale);

  Some(new_local)
}
