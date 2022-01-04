use rendiation_algebra::*;

use crate::*;

use super::HelperLineMesh;

pub struct CameraHelper {
  pub mesh: FatlineImpl,
}

impl CameraHelper {
  pub fn from_node_and_project_matrix(node: SceneNode, mat: Mat4<f32>) -> Self {
    let camera_mesh = build_debug_line_in_camera_space();
    let camera_mesh = FatlineMeshCellImpl::from(camera_mesh);
    let fatline_mat = FatLineMaterial::default().into_scene_material();
    let fatline_mat = MaterialCell::new(fatline_mat);
    let fatline = FatlineImpl::new(fatline_mat, camera_mesh, node);
    Self { mesh: fatline }
  }
}

fn build_debug_line_in_camera_space() -> HelperLineMesh {
  let near = 0.;
  let far = 1.;
  let left = 0.;
  let right = 1.;
  let top = 1.;
  let bottom = 0.;

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
      scene.resources.cameras.check_update_gpu(camera, gpu);

      for (_, camera) in &mut scene.cameras {
        scene.resources.cameras.check_update_gpu(camera, gpu);
      }

      let mut base = SceneMaterialRenderPrepareCtxBase {
        camera,
        pass_info: ctx.pass_info,
        resources: &mut scene.resources,
        pass: &DefaultPassDispatcher,
      };

      for (_, draw_camera) in &mut scene.cameras {
        let helper = self.helpers.get_or_insert_with(draw_camera, |draw_camera| {
          (
            CameraHelper::from_node_and_project_matrix(
              draw_camera.node.clone(),
              draw_camera.projection_matrix,
            ),
            |_, _| {}, // todo update
          )
        });
        helper.mesh.update(gpu, &mut base)
      }
    }
  }

  fn setup_pass<'a>(&'a self, pass: &mut SceneRenderPass<'a>, scene: &'a crate::Scene) {
    if !self.enabled {
      return;
    }

    let main_camera = scene
      .resources
      .cameras
      .get_unwrap(scene.active_camera.as_ref().unwrap());

    for (_, camera) in &scene.cameras {
      let helper = self.helpers.get_unwrap(camera);
      helper.mesh.setup_pass(pass, main_camera, &scene.resources);
    }
  }
}
