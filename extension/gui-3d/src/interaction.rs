use crate::*;

pub struct Interaction3dCtx<'a> {
  pub picker: &'a dyn Picker3d,
  pub mouse_world_ray: Ray3<f64>,
  pub normalized_mouse_position: Vec2<f32>, // (0, 1)
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
  ) -> Option<HitPoint3D<f64>>;

  fn pick_models_nearest(
    &self,
    models: &mut dyn Iterator<Item = EntityHandle<SceneModelEntity>>,
    world_ray: Ray3<f64>,
  ) -> Option<(HitPoint3D<f64>, EntityHandle<SceneModelEntity>)> {
    let mut nearest: Option<(HitPoint3D<f64>, EntityHandle<SceneModelEntity>)> = None;
    for m in models {
      if let Some(hit) = self.pick_model_nearest(m, world_ray) {
        if let Some(n) = nearest {
          if hit.is_near_than(&n.0) {
            nearest = Some((hit, m));
          }
        } else {
          nearest = Some((hit, m));
        }
      }
    }
    nearest
  }
}
