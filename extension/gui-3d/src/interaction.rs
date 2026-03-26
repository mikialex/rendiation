use crate::*;

pub struct Interaction3dCtx<'a> {
  pub picker: &'a dyn Picker3d,
  /// return nearest hit point for intersection_group
  pub world_ray_intersected_nearest: Option<(HitPoint3D<f64>, EntityHandle<SceneModelEntity>)>,
}

#[derive(Default)]
pub struct WidgetSceneModelIntersectionGroupConfig {
  pub group: FastHashSet<EntityHandle<SceneModelEntity>>,
}

pub trait Picker3d {
  fn pick_model_nearest(
    &self,
    model: EntityHandle<SceneModelEntity>,
    world_ray: Ray3<f64>,
  ) -> Option<MeshBufferHitPoint<f64>>;

  fn pick_model_all(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    world_ray: Ray3<f64>,
    results: &mut Vec<MeshBufferHitPoint<f64>>,
    local_result_scratch: &mut Vec<MeshBufferHitPoint<f32>>,
  ) -> Option<()>;

  fn pick_models_all(
    &self,
    models: &mut dyn Iterator<Item = EntityHandle<SceneModelEntity>>,
    world_ray: Ray3<f64>,
  ) -> (
    Vec<MeshBufferHitPoint<f64>>,
    Vec<EntityHandle<SceneModelEntity>>,
  );

  fn pick_models_nearest(
    &self,
    models: &mut dyn Iterator<Item = EntityHandle<SceneModelEntity>>,
    world_ray: Ray3<f64>,
  ) -> Option<(HitPoint3D<f64>, EntityHandle<SceneModelEntity>)>;
}
