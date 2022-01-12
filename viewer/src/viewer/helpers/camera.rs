use rendiation_algebra::*;
use rendiation_geometry::LineSegment;
use rendiation_renderable_mesh::{group::GroupedMesh, mesh::NoneIndexedMesh};

use crate::*;

use super::*;

pub struct CameraHelper {
  projection_cache: Mat4<f32>,
  model: HelperLineModel,
}

impl CameraHelper {
  pub fn from_node_and_project_matrix(node: SceneNode, project_mat: Mat4<f32>) -> Self {
    let camera_mesh = build_debug_line_in_camera_space(project_mat.inverse_or_identity());
    let camera_mesh = camera_mesh.into_resourced();
    let fatline_mat = FatLineMaterial {
      width: 3.,
      states: Default::default(),
    }
    .into_scene_material()
    .into_resourced();
    let fatline = HelperLineModel::new(fatline_mat, camera_mesh, node);
    Self {
      model: fatline,
      projection_cache: project_mat,
    }
  }

  pub fn update(&mut self, project_mat: Mat4<f32>) {
    if self.projection_cache != project_mat {
      let camera_mesh = build_debug_line_in_camera_space(project_mat.inverse_or_identity());
      *self.model.mesh = camera_mesh;
    }
  }
}

fn build_line_box(target: &mut Vec<LineSegment<Vec3<f32>>>, min: Vec3<f32>, max: Vec3<f32>) {
  //
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

  let near_left_down = Vec3::new(left, bottom, near);
  let near_left_top = Vec3::new(left, top, near);
  let near_right_down = Vec3::new(right, bottom, near);
  let near_right_top = Vec3::new(right, top, near);

  let far_left_down = Vec3::new(left, bottom, far);
  let far_left_top = Vec3::new(left, top, far);
  let far_right_down = Vec3::new(right, bottom, far);
  let far_right_top = Vec3::new(right, top, far);

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

  let lines = lines
    .iter()
    .map(|[start, end]| FatLineVertex {
      color: Vec4::new(1., 1., 1., 1.),
      start: *start * project_mat,
      end: *end * project_mat,
    })
    .collect();

  let lines = NoneIndexedMesh::new(lines);
  let lines = GroupedMesh::full(lines);
  HelperLineMesh::new(lines)
}

pub struct CameraHelpers {
  pub enabled: bool,
  pub helpers: ResourceMapper<CameraHelper, Camera>,
}

impl Default for CameraHelpers {
  fn default() -> Self {
    Self {
      enabled: true,
      helpers: Default::default(),
    }
  }
}

impl PassContent for CameraHelpers {
  fn update(
    &mut self,
    gpu: &rendiation_webgpu::GPU,
    scene: &mut crate::Scene,
    ctx: &PassUpdateCtx,
  ) {
    if !self.enabled {
      return;
    }

    if let Some(camera) = &mut scene.active_camera {
      scene
        .resources
        .content
        .cameras
        .check_update_gpu(camera, gpu);

      for (_, camera) in &mut scene.cameras {
        scene
          .resources
          .content
          .cameras
          .check_update_gpu(camera, gpu);
      }

      let mut base = SceneMaterialRenderPrepareCtxBase {
        camera,
        pass_info: ctx.pass_info,
        resources: &mut scene.resources.content,
        pass: &DefaultPassDispatcher,
      };

      for (_, draw_camera) in &mut scene.cameras {
        let helper = self.helpers.get_update_or_insert_with(
          draw_camera,
          |draw_camera| {
            CameraHelper::from_node_and_project_matrix(
              draw_camera.node.clone(),
              draw_camera.projection_matrix,
            )
          },
          |helper, camera| {
            helper.update(camera.projection_matrix);
          },
        );
        helper
          .model
          .update(gpu, &mut base, &mut scene.resources.scene)
      }
    }
  }

  fn setup_pass<'a>(&'a self, pass: &mut SceneRenderPass<'a>, scene: &'a crate::Scene) {
    if !self.enabled {
      return;
    }

    let main_camera = scene
      .resources
      .content
      .cameras
      .get_unwrap(scene.active_camera.as_ref().unwrap());

    for (_, camera) in &scene.cameras {
      let helper = self.helpers.get_unwrap(camera);
      helper.model.setup_pass(pass, main_camera, &scene.resources);
    }
  }
}
