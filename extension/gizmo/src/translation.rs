use crate::*;

pub fn use_translation_gizmo(cx: &mut UI3dCx) {
  use_inject_cx::<AxisActiveState>(cx, |cx| {
    use_provide_arrow_mesh_init(cx, |cx| {
      use_arrow_model(cx, AxisType::X);
      use_arrow_model(cx, AxisType::Y);
      use_arrow_model(cx, AxisType::Z);
    });
    use_provide_plane_mesh_init(cx, |cx| {
      use_plane_model(cx, AxisType::X);
      use_plane_model(cx, AxisType::Y);
      use_plane_model(cx, AxisType::Z);
    });

    cx.on_event(|_, _, cx| {
      if cx.message.get::<GizmoOutControl>().is_some() {
        access_cx_mut!(cx, axis, AxisActiveState);
        *axis = AxisActiveState::default()
      }

      if let Some(drag_action) = cx.message.get::<DragTargetAction>() {
        access_cx!(cx, target, Option::<GizmoControlTargetState>);
        access_cx!(cx, axis, AxisActiveState);
        access_cx!(cx, start_states, Option::<DragStartState>);

        if let Some(start_states) = start_states {
          if let Some(target) = target {
            if let Some(action) = handle_translating(start_states, target, axis, drag_action) {
              debug_print("handle translating");
              cx.message.put(GizmoUpdateTargetLocal(action))
            }
          }
        }
      }
    });
  })
}

fn use_provide_arrow_mesh_init(cx: &mut UI3dCx, f: impl FnOnce(&mut UI3dCx)) {
  fn create_arrow_mesh(cx: &mut UI3dBuildCx) -> AttributesMeshEntities {
    cx.writer
      .write_attribute_mesh(ArrowShape::default().build().build())
  }
  use_state_cx_in_mounting(cx, create_arrow_mesh, f)
}

fn use_arrow_model(cx: &mut UI3dCx, axis: AxisType) {
  state_pick(cx, axis_lens(axis), |cx| {
    use_axis_interactive_model(cx, axis, AxisType::mat)
  })
}

fn use_provide_plane_mesh_init(cx: &mut UI3dCx, f: impl FnOnce(&mut UI3dCx)) {
  let create_plane_mesh = build_attributes_mesh_by(|builder| {
    builder.triangulate_parametric(
      &ParametricPlane.transform3d_by(Mat4::translate((-0.5, -0.5, 0.))),
      TessellationConfig { u: 1, v: 1 },
      true,
    );
  });
  use_state_cx_in_mounting(cx, create_plane_mesh, f)
}

fn use_plane_model(cx: &mut UI3dCx, axis: AxisType) {
  access_cx!(cx.dyn_cx, gizmo, AxisActiveState);
  let (a, b) = gizmo.get_rest_axis(axis);
  let mut axis_state = ItemState {
    hovering: a.hovering && b.hovering,
    active: a.active && b.active,
  };

  inject_cx(cx, &mut axis_state, |cx| {
    use_axis_interactive_model(cx, axis, |axis| {
      let plane_scale = Mat4::scale(Vec3::splat(0.4));
      let plane_move = Vec3::splat(1.3);
      let degree_90 = f64::PI() / 2.;

      let move_dir = Vec3::one() - axis.dir();
      let move_mat = Mat4::translate(move_dir * plane_move);
      let rotate = match axis {
        AxisType::X => Mat4::rotate_y(degree_90),
        AxisType::Y => Mat4::rotate_x(-degree_90),
        AxisType::Z => Mat4::identity(),
      };
      move_mat * rotate * plane_scale
    });
  });

  access_cx_mut!(cx.dyn_cx, gizmo, AxisActiveState);
  let (a, b) = gizmo.get_rest_axis_mut(axis);
  // if the hovering state is not decided by one axis, then we override it
  // i think there is a better way to express this
  if a.hovering == b.hovering {
    a.hovering = axis_state.hovering;
    b.hovering = axis_state.hovering;
  }
  a.active |= axis_state.active; // the active will be correctly reset by stop dragging
  b.active |= axis_state.active;
}

fn handle_translating(
  states: &DragStartState,
  target: &GizmoControlTargetState,
  axis: &AxisActiveState,
  action: &DragTargetAction,
) -> Option<Mat4<f64>> {
  let camera_world_position = action.camera_world.position();

  let back_to_local = target.target_world_mat.inverse()?;
  let view_dir = camera_world_position - target.target_world_mat.position();
  let view_dir_in_local = view_dir.transform_direction(back_to_local).value;

  let plane_point = states.start_hit_local_position;

  // build world space constraint abstract interactive plane
  let (plane, constraint) = if axis.only_x_active() {
    Some((1., 0., 0.).into())
  } else if axis.only_y_active() {
    Some((0., 1., 0.).into())
  } else if axis.only_z_active() {
    Some((0., 0., 1.).into())
  } else {
    None
  }
  .map(|axis: Vec3<f64>| {
    let helper_dir = axis.cross(view_dir_in_local);
    let normal = helper_dir.cross(axis);
    (
      Plane::from_normal_and_plane_point(normal, plane_point),
      axis,
    )
  })
  .or_else(|| {
    if axis.only_xy_active() {
      Some((0., 0., 1.).into())
    } else if axis.only_yz_active() {
      Some((1., 0., 0.).into())
    } else if axis.only_xz_active() {
      Some((0., 1., 0.).into())
    } else {
      None
    }
    .map(|normal: Vec3<f64>| {
      (
        Plane::from_normal_and_plane_point(normal, plane_point),
        Vec3::one() - normal,
      )
    })
  })?;

  let local_ray = action.world_ray.apply_matrix_into(back_to_local);

  // if we don't get any hit, we skip update.  Keeping last updated result is a reasonable behavior.
  if let OptionalNearest(Some(new_hit)) = local_ray.intersect(&plane, &()) {
    let new_hit = (new_hit.position - plane_point) * constraint + plane_point;
    let new_hit_world = target.target_world_mat * new_hit;

    // new_hit_world = M(parent) * M(new_local_translate) * M(local_rotate) * M(local_scale) *
    // start_hit_local_position => M-1(parent) * new_hit_world = new_local_translate +
    // M(local_rotate) * M(local_scale) * start_hit_local_position  => new_local_translate =
    // M-1(parent) * new_hit_world - M(local_rotate) * M(local_scale) * start_hit_local_position

    let new_local_translate = (states.start_parent_world_mat.inverse()? * new_hit_world)
      - Mat4::from(states.start_local_quaternion)
        * Mat4::scale(states.start_local_scale)
        * states.start_hit_local_position;

    let new_local = Mat4::translate(new_local_translate)
      * Mat4::from(states.start_local_quaternion)
      * Mat4::scale(states.start_local_scale);

    Some(new_local)
  } else {
    None
  }
}
