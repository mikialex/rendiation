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
      if cx.message.get::<GizmoOutControl>().is_some() {
        access_cx_mut!(cx, axis, AxisActiveState);
        *axis = AxisActiveState::default()
      }

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

  plane_state_len(
    axis,
    UIWidgetModel::new(v, mesh)
      .with_parent(v, parent)
      .with_on_mouse_down(start_drag)
      .with_on_mouse_hovering(hovering)
      .with_on_mouse_out(stop_hovering)
      .into_view_independent(mat)
      .with_view_update(update_per_axis_model(axis)),
  )
}

struct PlaneStateLens<T> {
  inner: T,
  axis: AxisType,
}

fn plane_state_len(axis: AxisType, inner: impl Widget) -> PlaneStateLens<impl Widget> {
  PlaneStateLens { inner, axis }
}

fn plane_state_len_impl(axis: AxisType, f: impl FnOnce(&mut DynCx), cx: &mut DynCx) {
  access_cx!(cx, gizmo, AxisActiveState);
  let (a, b) = gizmo.get_rest_axis(axis);
  let mut axis_state = ItemState {
    hovering: a.hovering && b.hovering,
    active: a.active && b.active,
  };

  cx.scoped_cx(&mut axis_state, f);

  access_cx_mut!(cx, gizmo, AxisActiveState);
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

impl<T: Widget> Widget for PlaneStateLens<T> {
  fn update_state(&mut self, cx: &mut DynCx) {
    plane_state_len_impl(self.axis, move |cx| self.inner.update_state(cx), cx);
  }

  fn update_view(&mut self, cx: &mut DynCx) {
    plane_state_len_impl(self.axis, move |cx| self.inner.update_view(cx), cx);
  }

  fn clean_up(&mut self, cx: &mut DynCx) {
    self.inner.clean_up(cx);
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
