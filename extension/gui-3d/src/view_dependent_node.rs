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
      // we can only get last frame world matrix, so
      // we can only do view independent stuff in next frame.
      // if this is not acceptable, consider rerun the entire cx one more time.
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

    let state = e.get_viewport_pointer_ctx()?;

    config
      .scale
      .override_mat(
        origin_world,
        config.override_position,
        state.camera_world_mat,
        state.view_logical_pixel_size.y as f32,
        state.projection,
        state.projection_inv,
      )
      .into()
  });
}

pub fn use_billboard(
  cx: &mut UI3dCx,
  node: &EntityHandle<SceneNodeEntity>,
  mat: impl FnOnce() -> Mat4<f64> + 'static,
) {
  use_view_dependent_world_mat(cx, node, mat, |_, origin_world, e| {
    let state = e.get_viewport_pointer_ctx()?;
    BillBoard::default()
      .override_mat(origin_world, state.camera_world_mat.position())
      .into()
  });
}

pub fn use_view_dependent_world_mat(
  cx: &mut UI3dCx,
  node: &EntityHandle<SceneNodeEntity>,
  mat: impl FnOnce() -> Mat4<f64> + 'static,
  mat_updates: impl FnOnce(&mut DynCx, Mat4<f64>, &dyn WidgetEnvAccess) -> Option<Mat4<f64>>,
) {
  let (cx, origin_local_mat) = cx.use_plain_state(mat);
  let (cx, local_mat_to_sync) = cx.use_plain_state_default::<Option<Mat4<f64>>>();

  cx.on_event(|e, reader, cx| {
    let parent_world =
      if let Some(parent_node) = reader.node_reader.read::<SceneNodeParentIdx>(*node) {
        let parent_node = unsafe { EntityHandle::from_raw(parent_node) };
        e.widget_env.get_world_mat(parent_node).unwrap()
      } else {
        Mat4::identity()
      };

    let origin_world = parent_world * *origin_local_mat;
    if let Some(override_world_mat) = mat_updates(cx, origin_world, e.widget_env) {
      *local_mat_to_sync = Some(parent_world.inverse_or_identity() * override_world_mat);
    }
  });

  cx.on_update(|w, _| {
    if let Some(mat) = local_mat_to_sync.take() {
      w.set_local_matrix(*node, mat);
    }
  });
}
