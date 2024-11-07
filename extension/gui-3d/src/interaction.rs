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
  ) -> Option<HitPoint3D>;

  fn pick_models_nearest(
    &self,
    models: &mut dyn Iterator<Item = EntityHandle<SceneModelEntity>>,
    world_ray: Ray3,
  ) -> Option<(HitPoint3D, EntityHandle<SceneModelEntity>)> {
    let mut nearest: Option<(HitPoint3D, EntityHandle<SceneModelEntity>)> = None;
    for m in models {
      if let Some(hit) = self.pick_model_nearest(m, world_ray) {
        if let Some(n) = nearest {
          if hit.is_near_than(&n.0) {
            nearest = Some((hit, m));
          }
        }
      }
    }
    nearest
  }
}
