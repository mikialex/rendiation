use rendiation_controller::InputBound;

use crate::*;

pub struct ViewerViewPort {
  pub id: u64,
  /// x relative to surface top left, y relative to surface top left, width, height
  /// physical pixel unit
  pub viewport: Vec4<f32>,
  pub camera: EntityHandle<SceneCameraEntity>,
  pub camera_node: EntityHandle<SceneNodeEntity>,
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
  /// the order is preserved
  pub viewports_index: Vec<(usize, u64)>,
}

pub fn per_camera_per_viewport(
  cx: &mut ViewerCx,
  logic: impl Fn(&mut ViewerCx, &CameraViewportAccess),
) {
  let mut mapping = FastHashMap::<_, Vec<_>>::default();
  for (index, vp) in cx.viewer.content.viewports.iter().enumerate() {
    mapping
      .entry((vp.camera, vp.camera_node))
      .or_default()
      .push((index, vp.id));
  }
  for ((camera, camera_node), viewports) in mapping {
    let cv = CameraViewportAccess {
      camera,
      camera_node,
      viewports_index: viewports,
    };

    cx.keyed_scope(&camera, |cx| {
      logic(cx, &cv);
    });
  }
}
