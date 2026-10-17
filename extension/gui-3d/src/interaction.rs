use rendiation_scene_geometry_query::*;

use crate::*;

pub struct Interaction3dCtx<'a> {
  pub picker: &'a dyn Picker3d,
  /// return nearest hit point for intersection_group
  pub world_ray_intersected_nearest: Option<SceneRayHit>,
}

#[derive(Default)]
pub struct WidgetSceneModelIntersectionGroupConfig {
  pub group: FastHashSet<EntityHandle<SceneModelEntity>>,
}

pub trait Picker3d {
  /// ray query the given models, the scene level hit filter (for example clipping) is applied by
  /// the implementation
  fn ray_query_models(
    &self,
    models: &[EntityHandle<SceneModelEntity>],
    world_ray: Ray3<f64>,
    v: &mut dyn RayHitVisitor,
  );
}

pub trait Picker3dExt: Picker3d {
  fn pick_models_nearest(
    &self,
    models: &[EntityHandle<SceneModelEntity>],
    world_ray: Ray3<f64>,
  ) -> Option<SceneRayHit> {
    let mut v = NearestRayHit::default();
    self.ray_query_models(models, world_ray, &mut v);
    v.nearest
  }

  fn pick_model_nearest(
    &self,
    model: EntityHandle<SceneModelEntity>,
    world_ray: Ray3<f64>,
  ) -> Option<SceneRayHit> {
    self.pick_models_nearest(&[model], world_ray)
  }
}

impl<T: Picker3d + ?Sized> Picker3dExt for T {}
