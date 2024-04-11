use database::*;
use reactive::*;
use rendiation_algebra::*;
use rendiation_scene_core_next::*;

type CameraViewSceneModelAccess = (AllocIdx<SceneModelEntity>, AllocIdx<SceneCameraEntity>);

pub fn reactive_billboard_override_mat(
  base: impl ReactiveCollection<CameraViewSceneModelAccess, Mat4<f32>>,
  camera_view_mat: impl ReactiveCollection<AllocIdx<SceneCameraEntity>, Mat4<f32>>,
) -> impl ReactiveCollection<CameraViewSceneModelAccess, Mat4<f32>> {
  global_watch()
    .watch_typed_key::<SceneModelRotationOverride>()
    .collective_filter_map(|v| v) // todo, we should make this efficient in db level
    .collective_cross_join(camera_view_mat)
    .collective_zip(base)
    .collective_map(|((billboard, original_mat), camera_view)| {
      billboard.override_mat(original_mat, camera_view.position())
    })
    .materialize_unordered()
}

pub fn extend_scene_data_model() {
  global_entity_of::<SceneModelEntity>().declare_component::<SceneModelRotationOverride>();
}

declare_component!(
  SceneModelRotationOverride,
  SceneModelEntity,
  Option<BillBoard>
);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BillBoard {
  /// define what the front direction is (in object space)
  ///
  /// the front_direction will always lookat the view direction
  pub front_direction: Vec3<f32>,
}

impl BillBoard {
  pub fn override_mat(&self, world_matrix: Mat4<f32>, camera_position: Vec3<f32>) -> Mat4<f32> {
    let scale = world_matrix.get_scale();
    let scale = Mat4::scale(scale);
    let position = world_matrix.position();
    let position_m = Mat4::translate(position);

    let correction = Mat4::lookat(
      Vec3::new(0., 0., 0.),
      self.front_direction,
      Vec3::new(0., 1., 0.),
    );

    let rotation = Mat4::lookat(position, camera_position, Vec3::new(0., 1., 0.));

    // there must be cheaper ways
    position_m * rotation * correction * scale
  }
}

impl Default for BillBoard {
  fn default() -> Self {
    Self {
      front_direction: Vec3::new(0., 0., 1.),
    }
  }
}
