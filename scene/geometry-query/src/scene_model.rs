use crate::*;

pub trait SceneModelPicker {
  fn ray_query_nearest(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
  ) -> Option<MeshBufferHitPoint<f64>>;

  /// return None if errored
  /// todo, this should be improved
  fn ray_query_all(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
    results: &mut Vec<MeshBufferHitPoint<f64>>,
    local_result_scratch: &mut Vec<MeshBufferHitPoint<f32>>,
  ) -> Option<()>;
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

  fn ray_query_all(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
    results: &mut Vec<MeshBufferHitPoint<f64>>,
    local_result_scratch: &mut Vec<MeshBufferHitPoint<f32>>,
  ) -> Option<()> {
    for provider in self {
      if provider
        .ray_query_all(idx, ctx, results, local_result_scratch)
        .is_some()
      {
        return Some(());
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
  // keep result if return true
  pub filter: Option<Box<dyn Fn(&MeshBufferHitPoint<f64>, EntityHandle<SceneModelEntity>) -> bool>>,
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
      .apply_matrix_into(mat.inverse_or_identity()) // todo, cache inverse mat
      .into_f32();

    let hit = self
      .internal
      .ray_query_local_nearest(idx, local_ray, local_tolerance)?;

    let position = hit.hit.position.into_f64();
    let world_hit_position = position.apply_matrix_into(mat);

    let point = MeshBufferHitPoint {
      hit: HitPoint {
        position: world_hit_position,
        distance: ctx.world_ray.origin.distance_to(world_hit_position),
      },
      primitive_index: hit.primitive_index,
    };

    if let Some(filter) = self.filter.as_ref() {
      if !filter(&point, idx) {
        return None;
      }
    }

    point.into()
  }

  fn ray_query_all(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
    results: &mut Vec<MeshBufferHitPoint<f64>>,
    local_result_scratch: &mut Vec<MeshBufferHitPoint<f32>>,
  ) -> Option<()> {
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

    local_result_scratch.clear();

    self
      .internal
      .ray_query_local_all(idx, local_ray, local_tolerance, local_result_scratch)?;

    results.reserve(local_result_scratch.len());
    local_result_scratch
      .iter()
      .map(|r| transform_hit_point_to_world(*r, mat, ctx.world_ray.origin))
      .filter(|r| {
        if let Some(filter) = &self.filter {
          filter(r, idx)
        } else {
          true
        }
      })
      .for_each(|r| results.push(r));

    Some(())
  }
}

fn transform_hit_point_to_world(
  hit: MeshBufferHitPoint<f32>,
  world_mat: Mat4<f64>,
  camera_position: Vec3<f64>,
) -> MeshBufferHitPoint<f64> {
  let position = hit.hit.position.into_f64();
  let world_hit_position = position.apply_matrix_into(world_mat);

  MeshBufferHitPoint {
    hit: HitPoint {
      position: world_hit_position,
      distance: camera_position.distance_to(world_hit_position),
    },
    primitive_index: hit.primitive_index,
  }
}
