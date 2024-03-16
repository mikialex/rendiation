use std::sync::{Arc, RwLock};

use futures::{Stream, StreamExt};
use reactive::{PollUtils, SignalStreamExt, StreamMap};
use rendiation_algebra::*;
use rendiation_mesh_core::{GroupedMesh, NoneIndexedMesh};

use super::*;
use crate::WidenedLineVertex;

pub struct CameraHelper {
  projection_cache: Mat4<f32>,
  model: HelperLineModel,
}

impl CameraHelper {
  pub fn from_node_and_project_matrix(node: SceneNode, project_mat: Mat4<f32>) -> Self {
    let camera_mesh = build_debug_line_in_camera_space(project_mat.inverse_or_identity());
    let widened_line_mat = WidenedLineMaterial::new(3.);
    let widened_line = HelperLineModel::new(widened_line_mat, camera_mesh, &node);
    Self {
      model: widened_line,
      projection_cache: project_mat,
    }
  }

  pub fn update(&mut self, project_mat: Mat4<f32>) {
    if self.projection_cache != project_mat {
      let camera_mesh = build_debug_line_in_camera_space(project_mat.inverse_or_identity());
      self.model.update_mesh(camera_mesh);
    }
  }
}

fn line_box(min: Vec3<f32>, max: Vec3<f32>) -> impl IntoIterator<Item = [Vec3<f32>; 2]> {
  let near = min.x;
  let far = max.x;
  let left = min.z;
  let right = max.z;
  let top = max.y;
  let bottom = min.y;

  let near_left_down = Vec3::new(left, bottom, near);
  let near_left_top = Vec3::new(left, top, near);
  let near_right_down = Vec3::new(right, bottom, near);
  let near_right_top = Vec3::new(right, top, near);

  let far_left_down = Vec3::new(left, bottom, far);
  let far_left_top = Vec3::new(left, top, far);
  let far_right_down = Vec3::new(right, bottom, far);
  let far_right_top = Vec3::new(right, top, far);

  [
    [near_left_down, near_left_top],
    [near_right_down, near_right_top],
    [near_left_down, near_right_down],
    [near_left_top, near_right_top],
    //
    [far_left_down, far_left_top],
    [far_right_down, far_right_top],
    [far_left_down, far_right_down],
    [far_left_top, far_right_top],
    //
    [near_left_down, far_left_down],
    [near_left_top, far_left_top],
    [near_right_down, far_right_down],
    [near_right_top, far_right_top],
  ]
}

fn build_debug_line_in_camera_space(project_mat: Mat4<f32>) -> HelperLineMesh {
  let zero = 0.0001;
  let one = 0.9999;

  let near = zero;
  let far = one;
  let left = -one;
  let right = one;
  let top = one;
  let bottom = -one;

  let min = Vec3::new(near, left, bottom);
  let max = Vec3::new(far, right, top);

  let lines = line_box(min, max)
    .into_iter()
    .map(|[start, end]| WidenedLineVertex {
      color: Vec4::new(1., 1., 1., 1.),
      start: project_mat * start,
      end: project_mat * end,
    })
    .collect();

  let lines = NoneIndexedMesh::new(lines);
  let lines = GroupedMesh::full(lines);
  HelperLineMesh::new(lines)
}

pub type ReactiveCameraHelper = impl Stream<Item = ()> + AsRef<CameraHelper> + Unpin;

fn create_reactive_camera_helper(camera: &SceneCamera) -> ReactiveCameraHelper {
  let c = camera.read();
  let helper_init =
    CameraHelper::from_node_and_project_matrix(c.node.clone(), c.compute_project_mat());
  drop(c);
  let c = camera.downgrade();
  camera
    .unbound_listen_by(all_delta)
    .fold_signal(helper_init, move |delta, state| {
      match delta {
        SceneCameraImplDelta::bounds(_) => {}
        SceneCameraImplDelta::projection(_) => {
          if let Some(cam) = c.upgrade() {
            state.update(cam.read().compute_project_mat());
          }
        }
        SceneCameraImplDelta::node(_) => {
          if let Some(cam) = c.upgrade() {
            let c = cam.read();
            *state =
              CameraHelper::from_node_and_project_matrix(c.node.clone(), c.compute_project_mat());
          }
        }
      };
      Some(())
    })
}

pub struct CameraHelpers {
  /// this just control draw, it should be always get update
  pub enabled: bool,
  pub helpers: Arc<RwLock<StreamMap<u64, ReactiveCameraHelper>>>,
  updater: Box<dyn Stream<Item = ()> + Unpin>,
}

impl CameraHelpers {
  pub fn new(scene: &Scene) -> Self {
    let helpers: StreamMap<u64, ReactiveCameraHelper> = Default::default();
    let helpers = Arc::new(RwLock::new(helpers));
    let h = helpers.clone();
    let updater = scene
      .unbound_listen_by(all_delta)
      .filter_map_sync(|delta: MixSceneDelta| match delta {
        MixSceneDelta::cameras(c) => Some(c),
        _ => None,
      })
      .map(move |delta| match delta {
        // todo improve
        ContainerRefRetainContentDelta::Remove(c) => {
          let _ = helpers.write().unwrap().remove(c.guid()).unwrap();
        }
        ContainerRefRetainContentDelta::Insert(c) => {
          helpers
            .write()
            .unwrap()
            .insert(c.guid(), create_reactive_camera_helper(&c));
        }
      });
    Self {
      enabled: true,
      helpers: h,
      updater: Box::new(updater),
    }
  }
}

impl Stream for CameraHelpers {
  type Item = ();

  fn poll_next(
    mut self: __core::pin::Pin<&mut Self>,
    cx: &mut __core::task::Context<'_>,
  ) -> __core::task::Poll<Option<Self::Item>> {
    if self
      .updater
      .poll_until_pending_or_terminate_not_care_result(cx)
    {
      return __core::task::Poll::Ready(None);
    }
    self
      .helpers
      .write()
      .unwrap()
      .poll_until_pending_not_care_result(cx);
    __core::task::Poll::Pending
  }
}

impl PassContentWithSceneAndCamera for &mut CameraHelpers {
  fn render(
    &mut self,
    pass: &mut FrameRenderPass,
    scene: &SceneRenderResourceGroup,
    camera: &SceneCamera,
  ) {
    if !self.enabled {
      return;
    }

    for (_, draw_camera) in &scene.scene.cameras {
      let helpers = self.helpers.read().unwrap();
      let helper = helpers.get(&draw_camera.guid()).unwrap().as_ref();

      helper.model.inner.render(
        pass,
        &WidgetDispatcher::new(default_dispatcher(pass)),
        camera,
        scene,
      )
    }
  }
}
