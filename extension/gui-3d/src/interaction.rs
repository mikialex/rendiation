use crate::*;

pub struct Interaction3dCtx {
  pub picker: Box<dyn Picker3d>,
  pub mouse_world_ray: Ray3,
  /// return nearest hit point for intersection_group
  pub world_ray_intersected_nearest: Option<(HitPoint3D, EntityHandle<SceneModelEntity>)>,
}

#[derive(Default)]
pub struct WidgetSceneModelIntersectionGroupConfig {
  pub group: FastHashSet<EntityHandle<SceneModelEntity>>,
}

pub trait Picker3d {
  fn pick_model_nearest(
    &self,
    model: EntityHandle<SceneModelEntity>,
    world_ray: Ray3,
  ) -> Option<Vec3<f32>>;
}
