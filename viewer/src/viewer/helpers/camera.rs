use rendiation_algebra::*;

use crate::{
  FatLineMaterial, FatLineVertex, FatlineImpl, FatlineMeshCellImpl, MaterialCell, PassContent,
  SceneNode,
};

use super::HelperLineMesh;

pub struct CameraHelper {
  mesh: FatlineImpl,
}

impl CameraHelper {
  pub fn from_node_and_project_matrix(node: SceneNode, mat: Mat4<f32>) -> Self {
    let camera_mesh = build_debug_line_in_camera_space();
    let camera_mesh = FatlineMeshCellImpl::from(camera_mesh);
    let fatline_mat = FatLineMaterial::default();
    let fatline_mat = MaterialCell::new(fatline_mat);
    let fatline = FatlineImpl::new(fatline_mat, camera_mesh, node.clone());
    Self { mesh: fatline }
  }
}

fn build_debug_line_in_camera_space() -> HelperLineMesh {
  let near_left_down = Vec3::new(0., 0., 0.);
  let near_left_top = Vec3::new(0., 0., 0.);
  let near_right_down = Vec3::new(0., 0., 0.);
  let near_right_top = Vec3::new(0., 0., 0.);

  let far_left_down = Vec3::new(0., 0., 0.);
  let far_left_top = Vec3::new(0., 0., 0.);
  let far_right_down = Vec3::new(0., 0., 0.);
  let far_right_top = Vec3::new(0., 0., 0.);

  let lines = vec![
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
  ];

  let line_buffer = lines
    .iter()
    .map(|[start, end]| FatLineVertex {
      color: Vec4::new(1., 1., 1., 1.),
      start: *start,
      end: *end,
    })
    .collect();

  HelperLineMesh::new(line_buffer)
}

pub struct SceneCameraHelper {
  pub enabled: bool,
}

impl PassContent for SceneCameraHelper {
  fn update(
    &mut self,
    gpu: &rendiation_webgpu::GPU,
    scene: &mut crate::Scene,
    pass_info: &rendiation_webgpu::RenderPassInfo,
  ) {
    if !self.enabled {
      return;
    }
  }

  fn setup_pass<'a>(
    &'a self,
    pass: &mut rendiation_webgpu::GPURenderPass<'a>,
    scene: &'a crate::Scene,
  ) {
    if !self.enabled {
      return;
    }
  }
}
