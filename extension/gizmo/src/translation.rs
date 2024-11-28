use crate::*;

pub fn translation_gizmo_view(
  parent: EntityHandle<SceneNodeEntity>,
  v: &mut SceneWriter,
) -> impl Widget {
  WidgetGroup::default()
    .with_child(arrow(v, AxisType::X, parent))
    .with_child(arrow(v, AxisType::Y, parent))
    .with_child(arrow(v, AxisType::Z, parent))
    .with_child(plane(v, AxisType::X, parent))
    .with_child(plane(v, AxisType::Y, parent))
    .with_child(plane(v, AxisType::Z, parent))
    .with_state_post_update(|cx| {
      if let Some(drag_action) = cx.message.take::<DragTargetAction>() {
        access_cx!(cx, target, Option::<GizmoControlTargetState>);
        access_cx!(cx, axis, AxisActiveState);
        access_cx!(cx, start_states, Option::<DragStartState>);

        if let Some(start_states) = start_states {
          if let Some(target) = target {
            if let Some(action) = handle_translating(start_states, target, axis, drag_action) {
              println!("handle translating");
              cx.message.put(GizmoUpdateTargetLocal(action))
            }
          }
        }
      }
    })
    .with_local_state_inject(AxisActiveState::default())
}

fn arrow(
  v: &mut SceneWriter,
  axis: AxisType,
  parent: EntityHandle<SceneNodeEntity>,
) -> impl Widget {
  UIWidgetModel::new(v, ArrowShape::default().build())
    .with_parent(v, parent)
    .with_on_mouse_down(start_drag)
    .with_on_mouse_hovering(hovering)
    .with_on_mouse_out(stop_hovering)
    .into_view_independent(axis.mat())
    .with_view_update(update_per_axis_model(axis))
    .with_state_pick(axis_lens(axis))
}

fn plane(
  v: &mut SceneWriter,
  axis: AxisType,
  parent: EntityHandle<SceneNodeEntity>,
) -> impl Widget {
  let mesh = build_attributes_mesh(|builder| {
    builder.triangulate_parametric(
      &ParametricPlane.transform_by(Mat4::translate((-0.5, -0.5, 0.))),
      TessellationConfig { u: 1, v: 1 },
      true,
    );
  });

  fn plane_update(
    axis: AxisType,
  ) -> impl FnMut(&mut ViewIndependentWidgetModel, &mut DynCx) + 'static {
    move |plane, cx| {
      access_cx!(cx, style, GlobalUIStyle);
      let color = style.get_axis_primary_color(axis);

      access_cx!(cx, gizmo, AxisActiveState);
      let (a, b) = gizmo.get_rest_axis(axis);
      let axis_state = ItemState {
        hovering: a.hovering && b.hovering,
        active: a.active && b.active,
      };
      let self_active = axis_state.active;
      let visible = !gizmo.has_any_active() || self_active;
      let color = map_color(color, axis_state);
      access_cx_mut!(cx, cx3d, SceneWriter);
      plane.set_visible(cx3d, visible);
      plane.set_color(cx3d, color);
    }
  }

  let plane_scale = Mat4::scale(Vec3::splat(0.4));
  let plane_move = Vec3::splat(1.3);
  let degree_90 = f32::PI() / 2.;

  let move_dir = Vec3::one() - axis.dir();
  let move_mat = Mat4::translate(move_dir * plane_move);
  let rotate = match axis {
    AxisType::X => Mat4::rotate_y(degree_90),
    AxisType::Y => Mat4::rotate_x(-degree_90),
    AxisType::Z => Mat4::identity(),
  };
  let mat = move_mat * rotate * plane_scale;

  UIWidgetModel::new(v, mesh)
    .with_parent(v, parent)
    .with_on_mouse_down(start_drag)
    .with_on_mouse_hovering(plane_hovering(axis))
    .with_on_mouse_out(plane_stop_hovering(axis))
    .into_view_independent(mat)
    .with_view_update(plane_update(axis))
    .with_state_pick(axis_lens(axis))
}

fn plane_hovering(axis: AxisType) -> impl FnMut(&mut DynCx, HitPoint3D) {
  move |cx, _hit| {
    access_cx_mut!(cx, gizmo, AxisActiveState);
    let (a, b) = gizmo.get_rest_axis_mut(axis);
    a.hovering = true;
    b.hovering = true;
  }
}

fn plane_stop_hovering(axis: AxisType) -> impl FnMut(&mut DynCx) {
  move |cx| {
    access_cx_mut!(cx, gizmo, AxisActiveState);
    let (a, b) = gizmo.get_rest_axis_mut(axis);
    a.hovering = false;
    b.hovering = false;
  }
}

fn handle_translating(
  states: &DragStartState,
  target: &GizmoControlTargetState,
  axis: &AxisActiveState,
  action: DragTargetAction,
) -> Option<Mat4<f32>> {
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
  .map(|axis: Vec3<f32>| {
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
    .map(|normal: Vec3<f32>| {
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

    let new_local_translate = states.start_parent_world_mat.inverse()? * new_hit_world
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
