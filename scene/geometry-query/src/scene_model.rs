use crate::*;

pub trait SceneModelPicker {
  /// if the override_world_mat used, the internal node matrix is ignored
  fn ray_query_nearest(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    override_world_mat: Option<&Mat4<f64>>,
    ctx: &SceneRayQuery,
  ) -> Option<MeshBufferHitPoint<f64>>;

  /// if the override_world_mat used, the internal node matrix is ignored
  ///
  /// return None if errored
  /// todo, this should be improved
  fn ray_query_all(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    override_world_mat: Option<&Mat4<f64>>,
    ctx: &SceneRayQuery,
    results: &mut Vec<MeshBufferHitPoint<f64>>,
    local_result_scratch: &mut Vec<MeshBufferHitPoint<f32>>,
  ) -> Option<()>;
}

impl SceneModelPicker for Vec<Box<dyn SceneModelPicker>> {
  fn ray_query_nearest(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    override_world_mat: Option<&Mat4<f64>>,
    ctx: &SceneRayQuery,
  ) -> Option<MeshBufferHitPoint<f64>> {
    for provider in self {
      if let Some(hit) = provider.ray_query_nearest(idx, override_world_mat, ctx) {
        return Some(hit);
      }
    }
    None
  }

  fn ray_query_all(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    override_world_mat: Option<&Mat4<f64>>,
    ctx: &SceneRayQuery,
    results: &mut Vec<MeshBufferHitPoint<f64>>,
    local_result_scratch: &mut Vec<MeshBufferHitPoint<f32>>,
  ) -> Option<()> {
    for provider in self {
      if provider
        .ray_query_all(idx, override_world_mat, ctx, results, local_result_scratch)
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
  pub sm_world_bounding: BoxedDynQuery<EntityHandle<SceneModelEntity>, Box3<f64>>,
  pub sm_local_bounding: BoxedDynQuery<EntityHandle<SceneModelEntity>, Box3<f32>>,
  pub internal: T,
  // keep result if return true
  pub filter: Option<Box<dyn Fn(&MeshBufferHitPoint<f64>, EntityHandle<SceneModelEntity>) -> bool>>,
}

impl<T: LocalModelPicker> SceneModelPicker for SceneModelPickerBaseImpl<T> {
  fn ray_query_nearest(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    override_world_mat: Option<&Mat4<f64>>,
    ctx: &SceneRayQuery,
  ) -> Option<MeshBufferHitPoint<f64>> {
    let node = self.scene_model_node.get(idx)?;
    if !self.node_net_visible.access(&node)? {
      return None;
    }

    let (mat, sm_world_bounding) = if let Some(mat) = override_world_mat {
      let smb = self
        .sm_local_bounding
        .access(&idx)?
        .into_f64()
        .apply_matrix_into(*mat);
      (*mat, smb)
    } else {
      (
        self.node_world.access(&node)?,
        self.sm_world_bounding.access(&idx)?,
      )
    };

    let local_tolerance = pre_check_bounding_early_return_and_compute_local_tolerance(
      idx,
      ctx,
      sm_world_bounding,
      &self.internal,
      mat,
    )?;

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
    override_world_mat: Option<&Mat4<f64>>,
    ctx: &SceneRayQuery,
    results: &mut Vec<MeshBufferHitPoint<f64>>,
    local_result_scratch: &mut Vec<MeshBufferHitPoint<f32>>,
  ) -> Option<()> {
    let node = self.scene_model_node.get(idx)?;
    if !self.node_net_visible.access(&node)? {
      return None;
    }

    let (mat, sm_world_bounding) = if let Some(mat) = override_world_mat {
      let smb = self
        .sm_local_bounding
        .access(&idx)?
        .into_f64()
        .apply_matrix_into(*mat);
      (*mat, smb)
    } else {
      (
        self.node_world.access(&node)?,
        self.sm_world_bounding.access(&idx)?,
      )
    };

    let local_tolerance = pre_check_bounding_early_return_and_compute_local_tolerance(
      idx,
      ctx,
      sm_world_bounding,
      &self.internal,
      mat,
    )?;

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

fn pre_check_bounding_early_return_and_compute_local_tolerance(
  idx: EntityHandle<SceneModelEntity>,
  ctx: &SceneRayQuery,
  mut sm_world_bounding: Box3<f64>,
  internal: &impl LocalModelPicker,
  mat: Mat4<f64>,
) -> Option<f32> {
  let local_tolerance = if let Some(tolerance) = internal.bounding_enlarge_tolerance(idx)? {
    let target_world_center = sm_world_bounding.center();

    let local_tolerance =
      ctx.compute_local_tolerance(tolerance, mat.max_scale(), target_world_center);
    sm_world_bounding = sm_world_bounding.enlarge(local_tolerance as f64);
    local_tolerance
  } else {
    0.
  };

  let sm_intersected =
    IntersectAble::<_, bool, _>::intersect(&ctx.world_ray, &sm_world_bounding, &());

  if sm_intersected {
    return Some(local_tolerance);
  } else {
    None
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
