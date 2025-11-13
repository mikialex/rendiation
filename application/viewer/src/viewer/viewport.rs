use rendiation_controller::InputBound;

use crate::*;

pub struct ViewerViewPort {
  pub id: u64,
  /// x relative to surface top left, y relative to surface top left, width, height
  /// physical pixel unit
  pub viewport: Vec4<f32>,
  pub camera: EntityHandle<SceneCameraEntity>,
  pub camera_node: EntityHandle<SceneNodeEntity>,
  /// this camera is to debug the view related effect such as culling and lod selection
  /// for another camera.
  ///
  /// None as default, if None, then the view related effect compute is using the `camera`.
  pub debug_camera_for_view_related: Option<EntityHandle<SceneCameraEntity>>,
}

impl ViewerViewPort {
  /// lower than 1, 1 will round to 1, 1 to avoid render error
  pub fn render_pixel_size(&self) -> Size {
    Size::from_u32_pair_min_one(self.viewport.zw().map(|v| v as u32).into())
  }
}

pub fn find_top_hit<'a>(
  viewports: impl DoubleEndedIterator<Item = &'a ViewerViewPort>,
  mouse_position: (f32, f32),
) -> Option<(&'a ViewerViewPort, (f32, f32))> {
  let mut iter = viewports.rev();
  iter.find_map(|viewport| {
    let mouse_position_relative_to_viewport = Vec2::new(
      mouse_position.0 - viewport.viewport.x,
      mouse_position.1 - viewport.viewport.y,
    );

    let normalized_position_ndc = compute_normalized_position_in_canvas_coordinate(
      mouse_position_relative_to_viewport.into(),
      (viewport.viewport.z, viewport.viewport.w),
    );
    if normalized_position_ndc.0 >= -1.
      && normalized_position_ndc.1 >= -1.
      && normalized_position_ndc.0 <= 1.0
      && normalized_position_ndc.1 <= 1.0
    {
      Some((viewport, normalized_position_ndc))
    } else {
      None
    }
  })
}

pub fn viewport_to_input_bound(viewport: Vec4<f32>) -> InputBound {
  InputBound {
    origin: viewport.xy(),
    size: viewport.zw(),
  }
}

pub struct CameraViewportAccess {
  pub camera: EntityHandle<SceneCameraEntity>,
  pub camera_node: EntityHandle<SceneNodeEntity>,
  pub view_effect_camera: EntityHandle<SceneCameraEntity>,
  /// the order is preserved
  pub viewports_index: Vec<(usize, u64)>,
}

pub fn per_camera_per_viewport_scope(
  cx: &mut ViewerCx,
  consider_debug_view_camera_override: bool,
  logic: impl Fn(&mut ViewerCx, &CameraViewportAccess),
) {
  for cv in per_camera_per_viewport(
    &cx.viewer.content.viewports,
    consider_debug_view_camera_override,
  ) {
    cx.keyed_scope(&cv.camera, |cx| {
      logic(cx, &cv);
    });
  }
}

pub fn per_camera_per_viewport(
  view_ports: &[ViewerViewPort],
  consider_debug_view_camera_override: bool,
) -> impl Iterator<Item = CameraViewportAccess> {
  let mut mapping = FastHashMap::<_, Vec<_>>::default();
  for (index, vp) in view_ports.iter().enumerate() {
    let view_camera = if consider_debug_view_camera_override {
      vp.debug_camera_for_view_related.unwrap_or(vp.camera)
    } else {
      vp.camera
    };

    mapping
      .entry((view_camera, vp.camera, vp.camera_node))
      .or_default()
      .push((index, vp.id));
  }

  mapping
    .into_iter()
    .map(
      |((view_effect_camera, camera, camera_node), viewports)| CameraViewportAccess {
        camera,
        camera_node,
        view_effect_camera,
        viewports_index: viewports,
      },
    )
}
