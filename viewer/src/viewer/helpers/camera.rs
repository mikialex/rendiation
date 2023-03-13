use rendiation_algebra::*;
use rendiation_renderable_mesh::{group::GroupedMesh, mesh::NoneIndexedMesh};

use super::*;

pub struct CameraHelper {
  projection_cache: Mat4<f32>,
  model: HelperLineModel,
}

impl CameraHelper {
  pub fn from_node_and_project_matrix(node: SceneNode, project_mat: Mat4<f32>) -> Self {
    let camera_mesh = build_debug_line_in_camera_space(project_mat.inverse_or_identity());
    let fatline_mat = FatLineMaterial { width: 3. };
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

// pub struct CameraHelperSystem {
//   helpers: HashMap<usize, (CameraHelper, ChangeStream)>,
// }

// impl CameraHelperSystem {
//   pub fn get_with_update(&mut self, source: &SceneCamera) -> &CameraHelper {
//     let id = source.id();
//     let (mapped, changes) = self.gpu.entry(id).or_insert_with(|| {
//       let weak_to_remove = std::sync::Arc::downgrade(&self.to_remove);
//       source.drop_source().on(move |_| {
//         if let Some(to_remove) = weak_to_remove.upgrade() {
//           to_remove.write().unwrap().push(id);
//           false
//         } else {
//           true
//         }
//       });

//       (None, T::build_change_stream(source))
//     });

//     source.update(gpu_resource, changes, gpu)
//   }
// }

// struct CameraProjectionChange;
// type ChangeStream = impl Stream<Item = CameraProjectionChange>;

// impl CameraHelperSystem {
//   pub fn new(scene: &Scene) -> Self {
//     scene.listen_by(|view, send| {
//       match view {
//         Partial::All(_) => todo!(),
//         Partial::Delta(delta) => {
//           match delta {
//             SceneInnerDelta::cameras(camera) => {
//               //
//             }
//             _ => {}
//           }
//         }
//       }
//     });
//     todo!()
//   }

//   pub fn maintain(&mut self) {
//     //
//   }

//   pub fn render(&mut self) {
//     //
//   }
// }

pub struct CameraHelpers {
  pub enabled: bool,
  pub helpers: IdentityMapper<CameraHelper, SceneCameraInner>,
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
  fn render(&mut self, pass: &mut SceneRenderPass, scene: &SceneInner, camera: &SceneCamera) {
    if !self.enabled {
      return;
    }

    for (_, draw_camera) in &scene.cameras {
      let helper = self.helpers.get_update_or_insert_with(
        &draw_camera.read(),
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

      helper.model.inner.render(
        pass,
        &WidgetDispatcher::new(pass.default_dispatcher()),
        camera,
      )
    }
  }
}
