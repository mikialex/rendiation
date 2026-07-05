use rendiation_scene_geometry_query::*;

use crate::*;

/// compare to [SceneModelPickerBaseImpl], we do not do instance buffer entire bbox check.
/// this may be improved in the future
pub struct TransformInstancedMeshPicker<T> {
  pub internal: T,
  pub util: SceneModelPickerBaseImplUtil,
  pub instance_model: ForeignKeyReadView<SceneModelTransformInstancedModelPayload>,
  pub source_model: ForeignKeyReadView<TransformInstancedModelRefSceneModel>,
  pub per_unit_transform: ComponentReadView<TransformInstancedModelPerUnitTransform>,
  pub transform_buffer: ComponentReadView<TransformInstancedModelInstanceBuffer>,
}

impl<T> TransformInstancedMeshPicker<T> {
  fn get_view(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    override_world_mat: Option<&Mat4<f64>>,
    ignore_pre_check: bool,
  ) -> Option<TransformPickView<'_>> {
    let node = self.util.pre_check(idx, ignore_pre_check)?;
    let instance_own_transform = if let Some(mat) = override_world_mat {
      *mat
    } else {
      self.util.get_node_mat(node)?
    };

    let instance_model = self.instance_model.get(idx)?;
    let source_model = self.source_model.get(instance_model)?;
    let per_unit_transform = self.per_unit_transform.get(instance_model)?;
    let transforms = self.transform_buffer.get(instance_model)?.as_ref()?;
    Some(TransformPickView {
      transforms,
      per_unit_transform,
      source_model,
      instance_own_transform,
    })
  }
}

struct TransformPickView<'a> {
  transforms: &'a [Mat4<f32>],
  per_unit_transform: &'a Option<Mat4<f32>>,
  instance_own_transform: Mat4<f64>,
  source_model: EntityHandle<SceneModelEntity>,
}

impl<'a> TransformPickView<'a> {
  pub fn iter_mats(&'a self) -> impl Iterator<Item = Mat4<f64>> + 'a {
    self.transforms.iter().map(|m| {
      let mat = if let Some(per_unit_transform) = self.per_unit_transform {
        *m * *per_unit_transform
      } else {
        *m
      }
      .into_f64();
      self.instance_own_transform * mat
    })
  }
}

impl<T: SceneModelPicker> SceneModelPicker for TransformInstancedMeshPicker<T> {
  fn ray_query_nearest(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    override_world_mat: Option<&Mat4<f64>>,
    ctx: &SceneRayQuery,
    ignore_pre_check: bool,
  ) -> Option<MeshBufferHitPoint<f64>> {
    if let Some(internal) =
      self
        .internal
        .ray_query_nearest(idx, override_world_mat, ctx, ignore_pre_check)
    {
      return Some(internal);
    }
    let view = self.get_view(idx, override_world_mat, ignore_pre_check)?;

    let mut nearest: Option<MeshBufferHitPoint<f64>> = None;
    for m in view.iter_mats() {
      if let Some(h) = self
        .internal
        .ray_query_nearest(view.source_model, Some(&m), ctx, true)
      {
        let hit = h.hit;
        if let Some(n) = nearest {
          if hit.is_near_than(&n.hit) {
            nearest = Some(h);
          }
        } else {
          nearest = Some(h);
        }
      }
    }
    nearest
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
    if let Some(_) = self.internal.ray_query_all(
      idx,
      override_world_mat,
      ctx,
      results,
      local_result_scratch,
      ignore_pre_check,
    ) {
      return Some(());
    }
    let view = self.get_view(idx, override_world_mat, ignore_pre_check)?;
    for m in view.iter_mats() {
      if self
        .internal
        .ray_query_all(
          view.source_model,
          Some(&m),
          ctx,
          results,
          local_result_scratch,
          true,
        )
        .is_none()
      {
        return None;
      }
    }
    Some(())
  }

  fn frustum_query(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    override_world_mat: Option<&Mat4<f64>>,
    frustum: &SceneFrustumQuery,
    policy: ObjectTestPolicy,
    ignore_pre_check: bool,
  ) -> Option<bool> {
    if let Some(internal) =
      self
        .internal
        .frustum_query(idx, override_world_mat, frustum, policy, ignore_pre_check)
    {
      return Some(internal);
    }

    let view = self.get_view(idx, override_world_mat, ignore_pre_check)?;

    match policy {
      ObjectTestPolicy::Intersect => {
        for m in view.iter_mats() {
          if let Some(intersected) =
            self
              .internal
              .frustum_query(view.source_model, Some(&m), frustum, policy, true)
          {
            if intersected {
              return Some(true);
            }
          } else {
            return None;
          }
        }
        Some(false)
      }
      ObjectTestPolicy::Contains => {
        for m in view.iter_mats() {
          if let Some(contains) =
            self
              .internal
              .frustum_query(view.source_model, Some(&m), frustum, policy, true)
          {
            if !contains {
              return Some(false);
            }
          } else {
            return None;
          }
        }
        Some(true)
      }
    }
  }
}
