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

#[global_registered_collection]
pub fn camera_project_matrix() -> impl ReactiveCollection<AllocIdx<SceneCameraEntity>, Mat4<f32>> {
  let perspective = global_watch()
    .watch_typed_key::<SceneCameraPerspective>()
    .collective_filter_map(|proj| proj.map(|proj| proj.compute_projection_mat::<WebGPU>()));

  let orth = global_watch()
    .watch_typed_key::<SceneCameraOrthographic>()
    .collective_filter_map(|proj| proj.map(|proj| proj.compute_projection_mat::<WebGPU>()));

  perspective.collective_select(orth)
}
