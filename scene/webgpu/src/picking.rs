use crate::*;

pub struct SceneRayInteractiveCtx<'a> {
  pub world_ray: Ray3,
  pub conf: &'a MeshBufferIntersectConfig,
  pub node_derives: &'a SceneNodeDeriveSystem,
  pub camera: &'a SceneCamera,
  pub camera_view_size: Size,
}

pub trait SceneRayInteractive {
  fn ray_pick_nearest(&self, _ctx: &SceneRayInteractiveCtx) -> OptionalNearest<MeshBufferHitPoint>;
}

pub trait WebGPUScenePickingExt {
  fn build_interactive_ctx<'a>(
    &'a self,
    normalized_position: Vec2<f32>,
    camera_view_size: Size,
    conf: &'a MeshBufferIntersectConfig,
    node_derives: &'a SceneNodeDeriveSystem,
  ) -> SceneRayInteractiveCtx<'a>;

  fn interaction_picking<'a>(
    &'a self,
    ctx: &SceneRayInteractiveCtx,
    bounding_system: &mut SceneModelWorldBoundingSystem,
  ) -> Option<(&'a SceneModel, MeshBufferHitPoint)>;
}

use std::cmp::Ordering;

impl WebGPUScenePickingExt for SceneInner {
  fn build_interactive_ctx<'a>(
    &'a self,
    normalized_position: Vec2<f32>,
    camera_view_size: Size,
    conf: &'a MeshBufferIntersectConfig,
    node_derives: &'a SceneNodeDeriveSystem,
  ) -> SceneRayInteractiveCtx<'a> {
    let camera = self.active_camera.as_ref().unwrap();
    let world_ray = camera.cast_world_ray(normalized_position, node_derives);
    SceneRayInteractiveCtx {
      world_ray,
      conf,
      camera,
      camera_view_size,
      node_derives,
    }
  }

  fn interaction_picking<'a>(
    &'a self,
    ctx: &SceneRayInteractiveCtx,
    bounding_system: &mut SceneModelWorldBoundingSystem,
  ) -> Option<(&'a SceneModel, MeshBufferHitPoint)> {
    bounding_system.maintain();
    interaction_picking(
      self.models.iter().filter_map(|(handle, m)| {
        if let Some(bounding) = bounding_system.get_model_bounding(handle) {
          if ctx.world_ray.intersect(bounding, &()) {
            Some(m)
          } else {
            println!("culled");
            None
          }
        } else {
          // unbound model
          Some(m)
        }
      }),
      ctx,
    )
  }
}

pub fn interaction_picking<'a, T: IntoIterator<Item = &'a SceneModel>>(
  content: T,
  ctx: &SceneRayInteractiveCtx,
) -> Option<(&'a SceneModel, MeshBufferHitPoint)> {
  let mut result = Vec::new();

  for m in content {
    if let OptionalNearest(Some(r)) = m.ray_pick_nearest(ctx) {
      result.push((m, r));
    }
  }

  result.sort_by(|(_, a), (_, b)| {
    a.hit
      .distance
      .partial_cmp(&b.hit.distance)
      .unwrap_or(Ordering::Less)
  });

  result.into_iter().next()
}

pub enum HitReaction {
  // AnyHit(MeshBufferHitPoint),
  Nearest(MeshBufferHitPoint),
  None,
}

pub fn interaction_picking_mut<
  'a,
  X: SceneRayInteractive + ?Sized + 'a,
  T: IntoIterator<Item = &'a mut X>,
>(
  content: T,
  ctx: &SceneRayInteractiveCtx,
  mut cb: impl FnMut(&'a mut X, HitReaction),
) {
  let mut result = Vec::new();

  for m in content {
    if let OptionalNearest(Some(r)) = m.ray_pick_nearest(ctx) {
      // cb(m, HitReaction::AnyHit(r));
      result.push((m, r));
    } else {
      cb(m, HitReaction::None);
    }
  }

  result.sort_by(|(_, a), (_, b)| {
    a.hit
      .distance
      .partial_cmp(&b.hit.distance)
      .unwrap_or(Ordering::Less)
  });

  if let Some((m, r)) = result.into_iter().next() {
    cb(m, HitReaction::Nearest(r));
  }
}
