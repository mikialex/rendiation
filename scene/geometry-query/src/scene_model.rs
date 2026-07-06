use crate::*;

pub trait SceneModelPicker {
  /// if the override_world_mat used, the internal node matrix is ignored
  fn ray_query_nearest(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    override_world_mat: Option<&Mat4<f64>>,
    ctx: &SceneRayQuery,
    ignore_pre_check: bool,
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
    ignore_pre_check: bool,
  ) -> Option<()>;

  fn frustum_query(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    override_world_mat: Option<&Mat4<f64>>,
    frustum: &SceneFrustumQuery,
    policy: ObjectTestPolicy,
    ignore_pre_check: bool,
  ) -> Option<bool>;
}

#[derive(Clone, Copy, Debug)]
pub enum ObjectTestPolicy {
  Intersect,
  Contains,
}

impl<'a> SceneModelPicker for Box<dyn SceneModelPicker + 'a> {
  fn ray_query_nearest(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    override_world_mat: Option<&Mat4<f64>>,
    ctx: &SceneRayQuery,
    ignore_pre_check: bool,
  ) -> Option<MeshBufferHitPoint<f64>> {
    (**self).ray_query_nearest(idx, override_world_mat, ctx, ignore_pre_check)
  }

  fn ray_query_all(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    override_world_mat: Option<&Mat4<f64>>,
    ctx: &SceneRayQuery,
    results: &mut Vec<MeshBufferHitPoint<f64>>,
    local_result_scratch: &mut Vec<MeshBufferHitPoint<f32>>,
    ignore_pre_check: bool,
  ) -> Option<()> {
    (**self).ray_query_all(
      idx,
      override_world_mat,
      ctx,
      results,
      local_result_scratch,
      ignore_pre_check,
    )
  }

  fn frustum_query(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    override_world_mat: Option<&Mat4<f64>>,
    frustum: &SceneFrustumQuery,
    policy: ObjectTestPolicy,
    ignore_pre_check: bool,
  ) -> Option<bool> {
    (**self).frustum_query(idx, override_world_mat, frustum, policy, ignore_pre_check)
  }
}

impl SceneModelPicker for Vec<Box<dyn SceneModelPicker>> {
  fn ray_query_nearest(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    override_world_mat: Option<&Mat4<f64>>,
    ctx: &SceneRayQuery,
    ignore_pre_check: bool,
  ) -> Option<MeshBufferHitPoint<f64>> {
    for provider in self {
      if let Some(hit) = provider.ray_query_nearest(idx, override_world_mat, ctx, ignore_pre_check)
      {
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
    ignore_pre_check: bool,
  ) -> Option<()> {
    for provider in self {
      if provider
        .ray_query_all(
          idx,
          override_world_mat,
          ctx,
          results,
          local_result_scratch,
          ignore_pre_check,
        )
        .is_some()
      {
        return Some(());
      }
    }
    None
  }

  fn frustum_query(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    override_world_mat: Option<&Mat4<f64>>,
    frustum: &SceneFrustumQuery,
    policy: ObjectTestPolicy,
    ignore_pre_check: bool,
  ) -> Option<bool> {
    for provider in self {
      if let Some(r) =
        provider.frustum_query(idx, override_world_mat, frustum, policy, ignore_pre_check)
      {
        return Some(r);
      }
    }
    None
  }
}

#[derive(Clone)]
pub struct SceneModelPickerBaseImplUtil {
  pub node_world: BoxedDynQuery<EntityHandle<SceneNodeEntity>, Mat4<f64>>,
  pub node_net_visible: BoxedDynQuery<EntityHandle<SceneNodeEntity>, bool>,
  pub scene_model_node: ForeignKeyReadView<SceneModelRefNode>,
  pub selectable: ComponentReadView<SceneModelSelectable>,
}

impl SceneModelPickerBaseImplUtil {
  pub fn pre_check(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ignore_pre_check: bool,
  ) -> Option<EntityHandle<SceneNodeEntity>> {
    if ignore_pre_check {
      return Some(self.scene_model_node.get(idx)?);
    }

    if !self.selectable.get(idx).copied().unwrap() {
      return None;
    }
    let node = self.scene_model_node.get(idx)?;
    if !self.node_net_visible.access(&node)? {
      return None;
    }

    Some(node)
  }
  pub fn get_node_mat(&self, node: EntityHandle<SceneNodeEntity>) -> Option<Mat4<f64>> {
    self.node_world.access(&node)
  }
}

pub struct SceneModelPickerBaseImpl<T> {
  pub util: SceneModelPickerBaseImplUtil,
  pub sm_world_bounding: BoxedDynQuery<EntityHandle<SceneModelEntity>, Option<Box3<f64>>>,
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
    ignore_pre_check: bool,
  ) -> Option<MeshBufferHitPoint<f64>> {
    let node = self.util.pre_check(idx, ignore_pre_check)?;

    let (mat, sm_world_bounding) = if let Some(mat) = override_world_mat {
      let smb = self
        .sm_local_bounding
        .access(&idx)?
        .into_f64()
        .apply_matrix_into(*mat);
      (*mat, smb)
    } else {
      (
        self.util.get_node_mat(node)?,
        self.sm_world_bounding.access(&idx)??,
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

    let hit = self.internal.ray_query_local_nearest(
      idx,
      local_ray,
      local_tolerance,
      ctx.extra_screen_space_tolerance,
      &mat,
      &ctx.camera_ctx,
    )?;

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
    ignore_pre_check: bool,
  ) -> Option<()> {
    let node = self.util.pre_check(idx, ignore_pre_check)?;

    let (mat, sm_world_bounding) = if let Some(mat) = override_world_mat {
      let smb = self
        .sm_local_bounding
        .access(&idx)?
        .into_f64()
        .apply_matrix_into(*mat);
      (*mat, smb)
    } else {
      (
        self.util.get_node_mat(node)?,
        self.sm_world_bounding.access(&idx)??,
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

    self.internal.ray_query_local_all(
      idx,
      local_ray,
      local_tolerance,
      ctx.extra_screen_space_tolerance,
      local_result_scratch,
      &mat,
      &ctx.camera_ctx,
    )?;

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

  fn frustum_query(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    override_world_mat: Option<&Mat4<f64>>,
    ctx: &SceneFrustumQuery,
    policy: ObjectTestPolicy,
    ignore_pre_check: bool,
  ) -> Option<bool> {
    let node = self.util.pre_check(idx, ignore_pre_check)?;

    let (mat, _sm_world_bounding) = if let Some(mat) = override_world_mat {
      let smb = self
        .sm_local_bounding
        .access(&idx)?
        .into_f64()
        .apply_matrix_into(*mat);
      (*mat, smb)
    } else {
      (
        self.util.get_node_mat(node)?,
        self.sm_world_bounding.access(&idx)??,
      )
    };

    // todo, early return

    let frustum = ctx
      .world_frustum
      .apply_matrix_into(mat.inverse_or_identity());
    let frustum = frustum.into_f32();
    let helper = FrustumIntersectionTestHelper::new(&frustum);

    self.internal.frustum_query_local(
      idx,
      &frustum,
      helper.as_ref(),
      policy,
      ctx.extra_screen_space_tolerance,
      &mat,
      &ctx.camera_ctx,
    )
  }
}

fn pre_check_bounding_early_return_and_compute_local_tolerance(
  idx: EntityHandle<SceneModelEntity>,
  ctx: &SceneRayQuery,
  mut sm_world_bounding: Box3<f64>,
  internal: &impl LocalModelPicker,
  mat: Mat4<f64>,
) -> Option<f32> {
  let max_scale = mat.max_scale();
  let mut local_tolerance = if let Some(tolerance) = internal.bounding_enlarge_tolerance(idx)? {
    let target_world_center = sm_world_bounding.center();

    let local_tolerance =
      ctx
        .camera_ctx
        .compute_local_tolerance(tolerance, max_scale, target_world_center);
    local_tolerance
  } else {
    0.
  };

  if ctx.extra_screen_space_tolerance > 0. {
    let target_world_center = sm_world_bounding.center();
    local_tolerance += ctx.camera_ctx.compute_local_tolerance(
      IntersectTolerance::new(ctx.extra_screen_space_tolerance, ToleranceType::ScreenSpace),
      max_scale,
      target_world_center,
    );
  }

  if local_tolerance > 0. {
    sm_world_bounding = sm_world_bounding.enlarge(local_tolerance as f64 * max_scale);
  }

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
