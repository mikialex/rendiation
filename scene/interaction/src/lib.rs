use incremental::*;
use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_mesh_core::*;
use rendiation_scene_core::*;

mod agreement;
pub use agreement::*;

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
define_dyn_trait_downcaster_static!(SceneRayInteractive);

impl SceneRayInteractive for SceneModel {
  fn ray_pick_nearest(&self, ctx: &SceneRayInteractiveCtx) -> OptionalNearest<MeshBufferHitPoint> {
    self.visit(|model| model.ray_pick_nearest(ctx))
  }
}

impl SceneRayInteractive for SceneModelImpl {
  fn ray_pick_nearest(&self, ctx: &SceneRayInteractiveCtx) -> OptionalNearest<MeshBufferHitPoint> {
    ray_pick_nearest_core(self, ctx, ctx.node_derives.get_world_matrix(&self.node))
  }
}

pub fn ray_pick_nearest_core(
  m: &SceneModelImpl,
  ctx: &SceneRayInteractiveCtx,
  world_mat: Mat4<f32>,
) -> OptionalNearest<MeshBufferHitPoint> {
  match &m.model {
    ModelEnum::Standard(model) => {
      let net_visible = ctx.node_derives.get_net_visible(&m.node);
      if !net_visible {
        return OptionalNearest::none();
      }

      let world_inv = world_mat.inverse_or_identity();

      let local_ray = ctx.world_ray.clone().apply_matrix_into(world_inv);

      let model = model.read();

      if !model.material.is_keep_mesh_shape() {
        return OptionalNearest::none();
      }

      let mut result = model
        .mesh
        .intersect_nearest_by_group(local_ray, ctx.conf, model.group);

      // transform back to world space
      if let Some(result) = &mut result.0 {
        let hit = &mut result.hit;
        hit.position = world_mat * hit.position;
        hit.distance = (hit.position - ctx.world_ray.origin).length();
      };
      result
    }
    ModelEnum::Foreign(model) => {
      if let Some(model) =
        get_dyn_trait_downcaster_static!(SceneRayInteractive).downcast_ref(model.as_ref().as_any())
      {
        model.ray_pick_nearest(ctx)
      } else {
        OptionalNearest::none()
      }
    }
  }
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

impl WebGPUScenePickingExt for SceneCoreImpl {
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
