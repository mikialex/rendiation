use crate::*;

fn arrow() -> UIModel {
  UIModel::default().with_shape(ArrowShape::default().build())
}

pub fn translation_gizmo_view() -> impl View {
  let x_dir = Mat4::rotate_z(-f32::PI() / 2.);
  let x_arrow = arrow()
    .with_matrix(x_dir)
    .with_view_update(arrow_update(AxisType::X))
    .with_on_mouse_down(arrow_mouse_down());

  let y_dir = Mat4::identity();
  let y_arrow = arrow().with_matrix(y_dir);

  let z_dir = Mat4::rotate_x(f32::PI() / 2.);
  let z_arrow = arrow().with_matrix(z_dir);

  // let (axises_view, active_state) = AxisActiveState::state_provider();

  // let (axis_view, x_state) = axises_view.sub_state_provider(active_state);

  UIGroup::default()
    .with_child(x_arrow)
    .with_child(y_arrow)
    .with_child(z_arrow)
}

fn arrow_update(axis: AxisType) -> impl FnMut(&mut UIModel, &mut StateReadStore) + 'static {
  let gizmo = StateTag::<AxisActiveState>::default();
  let global_style = StateTag::<GlobalUIStyle>::default();
  move |arrow, model| {
    let color = model.state(&global_style, |style| style.get_axis_primary_color(axis));

    model.state(&gizmo, |gizmo| {
      let axis_state = *gizmo.get_axis(axis);
      let self_active = axis_state.active;
      arrow.set_visible(!gizmo.has_any_active() || self_active);
      arrow.set_color(map_color(color, axis_state));
    });
  }
}

fn arrow_mouse_down() -> impl FnMut(&mut View3dStateUpdateCtx, Vec3<f32>) + 'static {
  let state = StateTag::<ItemState>::default();
  move |cx, pick_position| {
    cx.state.state(&state, |state| state.active = true);
  }
}

fn handle_translating(
  states: &StartState,
  target: &TargetState,
  active: &AxisActiveState,
  action: DragTargetAction,
) -> Option<Mat4<f32>> {
  let camera_world_position = action.camera_world.position();

  let back_to_local = target.target_world_mat.inverse()?;
  let view_dir = camera_world_position - target.target_world_mat.position();
  let view_dir_in_local = view_dir.transform_direction(back_to_local).value;

  let plane_point = states.start_hit_local_position;

  // build world space constraint abstract interactive plane
  let (plane, constraint) = if active.only_x_active() {
    Some((1., 0., 0.).into())
  } else if active.only_y_active() {
    Some((0., 1., 0.).into())
  } else if active.only_z_active() {
    Some((0., 0., 1.).into())
  } else {
    None
  }
  .map(|axis: Vec3<f32>| {
    let helper_dir = axis.cross(view_dir_in_local);
    let normal = helper_dir.cross(axis);
    (
      Plane::from_normal_and_plane_point(normal, plane_point),
      axis,
    )
  })
  .or_else(|| {
    if active.only_xy_active() {
      Some((0., 0., 1.).into())
    } else if active.only_yz_active() {
      Some((1., 0., 0.).into())
    } else if active.only_xz_active() {
      Some((0., 1., 0.).into())
    } else {
      None
    }
    .map(|normal: Vec3<f32>| {
      (
        Plane::from_normal_and_plane_point(normal, plane_point),
        Vec3::one() - normal,
      )
    })
  })?;

  let local_ray = action.world_ray.apply_matrix_into(back_to_local);

  //   // if we don't get any hit, we skip update.  Keeping last updated result is a reasonable
  // behavior.   if let OptionalNearest(Some(new_hit)) = local_ray.intersect(&plane, &()) {
  //     let new_hit = (new_hit.position - plane_point) * constraint + plane_point;
  //     let new_hit_world = target.target_world_mat * new_hit;

  //     #[rustfmt::skip]
  //     // new_hit_world = M(parent) * M(new_local_translate) * M(local_rotate) * M(local_scale) *
  // start_hit_local_position =>     // M-1(parent) * new_hit_world = new_local_translate +
  // M(local_rotate) * M(local_scale) * start_hit_local_position  =>     // new_local_translate =
  // M-1(parent) * new_hit_world - M(local_rotate) * M(local_scale) * start_hit_local_position

  //     let new_local_translate = states.start_parent_world_mat.inverse()? * new_hit_world
  //       - Mat4::from(states.start_local_quaternion)
  //         * Mat4::scale(states.start_local_scale)
  //         * states.start_hit_local_position;

  //     let new_local = Mat4::translate(new_local_translate)
  //       * Mat4::from(states.start_local_quaternion)
  //       * Mat4::scale(states.start_local_scale);

  //     Some(new_local)
  //   } else {
  //     None
  //   }

  todo!()
}
