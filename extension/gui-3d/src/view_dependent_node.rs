use crate::*;

pub fn use_view_independent_scale_root<R>(
  cx: &mut UI3dCx,
  node: &EntityHandle<SceneNodeEntity>,
  config: ViewAutoScalable,
  inner: impl Fn(&mut UI3dCx) -> R,
) -> R {
  let mut computer = None;
  cx.on_event(|e, _, cx| unsafe {
    computer = ViewIndependentScaleCx {
      override_position: e.widget_env.get_world_mat(*node).unwrap().position(),
      scale: config,
    }
    .into();
    cx.register_cx(&mut computer);
  });

  let r = inner(cx);

  cx.on_event(|_, _, cx| unsafe {
    cx.unregister_cx::<Option<ViewIndependentScaleCx>>();
  });

  r
}

struct ViewIndependentScaleCx {
  override_position: Vec3<f64>,
  scale: ViewAutoScalable,
}

pub fn use_view_independent_scale_node(
  cx: &mut UI3dCx,
  node: &EntityHandle<SceneNodeEntity>,
  mat: impl FnOnce() -> Mat4<f64> + 'static,
) {
  use_view_dependent_world_mat(cx, node, mat, |cx, origin_world, e| {
    access_cx!(cx, config, Option<ViewIndependentScaleCx>);
    let config = config.as_ref().unwrap();

    config.scale.override_mat(
      origin_world,
      config.override_position,
      e.get_camera_world_mat(),
      e.get_view_resolution().y as f32,
      e.get_camera_perspective_proj(),
    )
  });
}

pub fn use_billboard(
  cx: &mut UI3dCx,
  node: &EntityHandle<SceneNodeEntity>,
  mat: impl FnOnce() -> Mat4<f64> + 'static,
) {
  use_view_dependent_world_mat(cx, node, mat, |_, origin_world, e| {
    BillBoard::default().override_mat(origin_world, e.get_camera_world_mat().position())
  });
}

pub fn use_view_dependent_world_mat(
  cx: &mut UI3dCx,
  node: &EntityHandle<SceneNodeEntity>,
  mat: impl FnOnce() -> Mat4<f64> + 'static,
  mat_updates: impl FnOnce(&mut DynCx, Mat4<f64>, &dyn WidgetEnvAccess) -> Mat4<f64>,
) {
  let (cx, origin_local_mat) = cx.use_plain_state_init(|_| mat());
  let (cx, local_mat_to_sync) = cx.use_plain_state::<Option<Mat4<f64>>>();

  cx.on_event(|e, reader, cx| {
    let parent_world =
      if let Some(parent_node) = reader.node_reader.read::<SceneNodeParentIdx>(*node) {
        let parent_node = unsafe { EntityHandle::from_raw(parent_node) };
        // todo, now we can only get last frame world matrix, so
        // we can only do view independent stuff in next frame.
        e.widget_env.get_world_mat(parent_node).unwrap()
      } else {
        Mat4::identity()
      };

    let origin_world = parent_world * *origin_local_mat;
    let override_world_mat = mat_updates(cx, origin_world, e.widget_env);

    *local_mat_to_sync = Some(parent_world.inverse_or_identity() * override_world_mat);
  });

  cx.on_update(|w, _| {
    if let Some(mat) = local_mat_to_sync.take() {
      w.set_local_matrix(*node, mat);
    }
  });
}
