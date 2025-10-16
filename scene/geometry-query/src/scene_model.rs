use crate::*;

pub trait SceneModelPicker {
  fn ray_query_nearest(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
  ) -> Option<MeshBufferHitPoint<f64>>;
}

impl SceneModelPicker for Vec<Box<dyn SceneModelPicker>> {
  fn ray_query_nearest(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
  ) -> Option<MeshBufferHitPoint<f64>> {
    for provider in self {
      if let Some(hit) = provider.ray_query_nearest(idx, ctx) {
        return Some(hit);
      }
    }
    None
  }
}

pub struct SceneModelPickerBaseImpl<T> {
  pub node_world: BoxedDynQuery<EntityHandle<SceneNodeEntity>, Mat4<f64>>,
  pub node_net_visible: BoxedDynQuery<EntityHandle<SceneNodeEntity>, bool>,
  pub scene_model_node: ForeignKeyReadView<SceneModelRefNode>,
  pub internal: T,
}

impl<T: LocalModelPicker> SceneModelPicker for SceneModelPickerBaseImpl<T> {
  fn ray_query_nearest(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
  ) -> Option<MeshBufferHitPoint<f64>> {
    let node = self.scene_model_node.get(idx)?;
    if !self.node_net_visible.access(&node)? {
      return None;
    }

    let mat = self.node_world.access(&node)?;
    let local_tolerance = self.internal.compute_local_tolerance(idx, ctx, mat)?;

    if !self.internal.bounding_pre_test(idx, ctx, local_tolerance)? {
      return None;
    }

    let local_ray = ctx
      .world_ray
      .apply_matrix_into(mat.inverse_or_identity())
      .into_f32();

    let hit = self
      .internal
      .ray_query_local_nearest(idx, local_ray, local_tolerance)?;

    let position = hit.hit.position.into_f64();
    let world_hit_position = position.apply_matrix_into(mat);

    MeshBufferHitPoint {
      hit: HitPoint {
        position: world_hit_position,
        distance: ctx.world_ray.origin.distance_to(world_hit_position),
      },
      primitive_index: hit.primitive_index,
    }
    .into()
  }
}
