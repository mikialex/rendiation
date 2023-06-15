use futures::Stream;
use reactive::{do_updates, ReactiveMap};
use rendiation_algebra::*;
use rendiation_renderable_mesh::{group::GroupedMesh, mesh::NoneIndexedMesh};

use super::*;
use crate::FatLineVertex;

pub struct CameraHelper {
  projection_cache: Mat4<f32>,
  model: HelperLineModel,
}

impl CameraHelper {
  pub fn from_node_and_project_matrix(node: SceneNode, project_mat: Mat4<f32>) -> Self {
    let camera_mesh = build_debug_line_in_camera_space(project_mat.inverse_or_identity());
    let fatline_mat = FatLineMaterial::new(3.);
    let fatline = HelperLineModel::new(fatline_mat, camera_mesh, &node);
    Self {
      model: fatline,
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
    .map(|[start, end]| FatLineVertex {
      color: Vec4::new(1., 1., 1., 1.),
      start: project_mat * start,
      end: project_mat * end,
    })
    .collect();

  let lines = NoneIndexedMesh::new(lines);
  let lines = GroupedMesh::full(lines);
  HelperLineMesh::new(lines)
}

impl SceneItemReactiveMapping<CameraHelper> for SceneCamera {
  type ChangeStream = impl Stream<Item = Mat4<f32>> + Unpin;
  type Ctx<'a> = ();

  fn build(&self, _: &Self::Ctx<'_>) -> (CameraHelper, Self::ChangeStream) {
    let source = self.read();
    let helper =
      CameraHelper::from_node_and_project_matrix(source.node.clone(), source.compute_project_mat());

    // todo, node change
    let change = self.create_projection_mat_stream();
    (helper, change)
  }

  fn update(&self, mapped: &mut CameraHelper, change: &mut Self::ChangeStream, _: &Self::Ctx<'_>) {
    do_updates(change, |delta| {
      mapped.update(delta);
    });
  }
}

pub struct CameraHelpers {
  pub enabled: bool,
  pub helpers: ReactiveMap<SceneCamera, CameraHelper>,
}

impl Default for CameraHelpers {
  fn default() -> Self {
    Self {
      enabled: true,
      helpers: Default::default(),
    }
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
      let helper = self.helpers.get_with_update(draw_camera, &());

      helper.model.inner.render(
        pass,
        &WidgetDispatcher::new(default_dispatcher(pass)),
        camera,
        scene,
      )
    }
  }
}
