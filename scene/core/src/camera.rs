use crate::*;

declare_entity!(SceneCameraEntity);
declare_foreign_key!(SceneCameraBelongsToScene, SceneCameraEntity, SceneEntity);
declare_foreign_key!(SceneCameraNode, SceneCameraEntity, SceneNodeEntity);

declare_component!(
  SceneCameraPerspective,
  SceneCameraEntity,
  Option<PerspectiveProjection<f32>>
);

declare_component!(
  SceneCameraOrthographic,
  SceneCameraEntity,
  Option<OrthographicProjection<f32>>
);

pub fn register_camera_data_model() {
  global_database()
    .declare_entity::<SceneCameraEntity>()
    .declare_component::<SceneCameraPerspective>()
    .declare_component::<SceneCameraOrthographic>()
    .declare_foreign_key::<SceneCameraBelongsToScene>()
    .declare_foreign_key::<SceneCameraNode>();
}

pub fn camera_project_matrix(
  ndc_mapper: impl NDCSpaceMapper<f32> + Copy,
) -> impl ReactiveQuery<Key = EntityHandle<SceneCameraEntity>, Value = Mat4<f32>> {
  let perspective = global_watch()
    .watch::<SceneCameraPerspective>()
    .collective_filter_map(move |proj| proj.map(|proj| proj.compute_projection_mat(&ndc_mapper)));

  let orth = global_watch()
    .watch::<SceneCameraOrthographic>()
    .collective_filter_map(move |proj| proj.map(|proj| proj.compute_projection_mat(&ndc_mapper)));

  perspective.collective_select(orth)
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct CameraTransform {
  pub projection: Mat4<f32>,
  pub projection_inv: Mat4<f32>,

  pub rotation: Mat4<f32>,

  pub view: Mat4<f32>,
  pub world: Mat4<f32>,

  pub view_projection: Mat4<f32>,
  pub view_projection_inv: Mat4<f32>,
}

impl CameraTransform {
  pub fn new(proj: Mat4<f32>, world: Mat4<f32>) -> Self {
    let view = world.inverse_or_identity();
    let view_projection = proj * view;
    CameraTransform {
      world,
      view,
      rotation: world.extract_rotation_mat(),

      projection: proj,
      projection_inv: proj.inverse_or_identity(),
      view_projection,
      view_projection_inv: view_projection.inverse_or_identity(),
    }
  }
}

/// normalized_position: -1 to 1
pub fn cast_world_ray(view_projection_inv: Mat4<f32>, normalized_position: Vec2<f32>) -> Ray3<f32> {
  let start = Vec3::new(normalized_position.x, normalized_position.y, -0.5);
  let end = Vec3::new(normalized_position.x, normalized_position.y, 0.5);

  let world_start = view_projection_inv * start;
  let world_end = view_projection_inv * end;

  Ray3::from_origin_to_target(world_start, world_end)
}

pub fn camera_transforms(
  ndc_mapper: impl NDCSpaceMapper<f32> + Copy,
) -> impl ReactiveQuery<Key = EntityHandle<SceneCameraEntity>, Value = CameraTransform> {
  let projections = camera_project_matrix(ndc_mapper);
  let node_mats = scene_node_derive_world_mat();

  let camera_world_mat =
    node_mats.one_to_many_fanout(global_rev_ref().watch_inv_ref::<SceneCameraNode>());

  camera_world_mat
    .collective_zip(projections)
    .collective_map(|(world, proj)| CameraTransform::new(proj, world))
}
