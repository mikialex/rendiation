use crate::*;

pub fn rotation_gizmo_view(parent: AllocIdx<SceneNodeEntity>) -> impl View {
  UIGroup::default()
}

pub fn build_rotator(axis: AxisType, parent: AllocIdx<SceneNodeEntity>) -> impl View {
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

  UIWidgetModel::default()
    .with_shape(mesh)
    .with_parent(parent)
    .with_matrix(mat)
    // .with_view_update(plane_update(AxisType::X))
    .with_state_pick(axis_lens(axis))
}

struct RotateState {
  current_angle_all: f32,
  last_dir: Vec2<f32>,
}

fn handle_rotating(
  states: &StartState,
  target: &TargetState,
  rotate_state: &mut Option<RotateState>,
  rotate_view: &AxisActiveState,
  action: DragTargetAction,
) -> Option<Mat4<f32>> {
  #[rustfmt::skip]
    // // new_hit_world = M(parent) * M(local_translate) * M(new_local_rotate) * M(local_scale) * start_hit_local_position =>
    // //  M-1(local_translate) * M-1(parent) * new_hit_world =  M(new_local_rotate) * M(local_scale) * start_hit_local_position
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

  let axis = if rotate_view.only_x_active() {
    Vec3::new(1., 0., 0.)
  } else if rotate_view.only_y_active() {
    Vec3::new(0., 1., 0.)
  } else if rotate_view.only_z_active() {
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
