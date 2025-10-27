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

fn viewport_to_input_bound(viewport: Vec4<f32>) -> InputBound {
  InputBound {
    origin: viewport.xy(),
    size: viewport.zw(),
  }
}

pub struct CameraViewportAccess {
  pub camera: EntityHandle<SceneCameraEntity>,
  pub camera_node: EntityHandle<SceneNodeEntity>,
  pub viewports_index: Vec<(usize, u64)>,
}

pub fn per_camera_per_viewport(
  cx: &mut ViewerCx,
  logic: impl Fn(&mut ViewerCx, &CameraViewportAccess),
) {
  let mut mapping = FastHashMap::<_, Vec<_>>::default();
  for (index, vp) in cx.viewer.scene.viewports.iter().enumerate() {
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
