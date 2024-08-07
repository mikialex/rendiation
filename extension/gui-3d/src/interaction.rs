use crate::*;

pub struct Interaction3dCtx {
  pub picker: Box<dyn Picker3d>,
  pub mouse_world_ray: Ray3,
  pub intersection_group: FastHashSet<EntityHandle<SceneModelEntity>>,
  /// return each model nearest hit point, sorted by distance
  pub world_ray_intersected_nearest: Option<(HitPoint3D, EntityHandle<SceneModelEntity>)>,
}

pub trait Picker3d {
  fn pick_model_nearest(
    &self,
    model: EntityHandle<SceneModelEntity>,
    world_ray: Ray3,
  ) -> Option<Vec3<f32>>;
}
