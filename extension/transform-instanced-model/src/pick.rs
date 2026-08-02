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
      self.util.get_mat_and_world_aabb(node, idx)?.0
    };

    let instance_model = self.instance_model.get(idx)?;
    let source_model = self.source_model.get(instance_model)?;
    let per_unit_transform = self.per_unit_transform.get(instance_model)?;
    let transforms = self.transform_buffer.get(instance_model)?.as_ref();
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
    request: SceneModelRayNearestQueryRequest,
  ) -> Option<MeshBufferHitPoint<f64>> {
    if let Some(internal) = self.internal.ray_query_nearest(request) {
      return Some(internal);
    }
    let view = self.get_view(
      request.idx,
      request.override_world_mat,
      request.ignore_pre_check,
    )?;

    let mut nearest: Option<MeshBufferHitPoint<f64>> = None;
    for (i, m) in view.iter_mats().enumerate() {
      if let Some(mut h) = self
        .internal
        .ray_query_nearest(SceneModelRayNearestQueryRequest {
          idx: view.source_model,
          override_world_mat: Some(&m),
          ctx: request.ctx,
          ignore_pre_check: true,
        })
      {
        h.primitive_index = i;
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

  fn ray_query_all(&self, request: SceneModelRayAllQueryRequest) -> Option<()> {
    if let Some(_) = self.internal.ray_query_all(SceneModelRayAllQueryRequest {
      results: request.results,
      local_result_scratch: request.local_result_scratch,
      ..request
    }) {
      return Some(());
    }
    let view = self.get_view(
      request.idx,
      request.override_world_mat,
      request.ignore_pre_check,
    )?;
    for (i, m) in view.iter_mats().enumerate() {
      let start = request.results.len();
      let internal_test = self.internal.ray_query_all(SceneModelRayAllQueryRequest {
        idx: view.source_model,
        override_world_mat: Some(&m),
        results: request.results,
        local_result_scratch: request.local_result_scratch,
        ignore_pre_check: true,
        ..request
      });

      for r in &mut request.results[start..] {
        r.primitive_index = i;
      }

      if internal_test.is_none() {
        return None;
      }
    }
    Some(())
  }

  fn frustum_query(&self, request: SceneModelFrustumQueryRequest) -> Option<bool> {
    if let Some(internal) = self.internal.frustum_query(request) {
      return Some(internal);
    }

    let view = self.get_view(
      request.idx,
      request.override_world_mat,
      request.ignore_pre_check,
    )?;

    match request.policy {
      ObjectTestPolicy::Intersect => {
        for m in view.iter_mats() {
          if let Some(intersected) = self.internal.frustum_query(SceneModelFrustumQueryRequest {
            idx: view.source_model,
            override_world_mat: Some(&m),
            policy: ObjectTestPolicy::Intersect,
            ignore_pre_check: true,
            ..request
          }) {
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
          if let Some(contains) = self.internal.frustum_query(SceneModelFrustumQueryRequest {
            idx: view.source_model,
            override_world_mat: Some(&m),
            policy: ObjectTestPolicy::Contains,
            ignore_pre_check: true,
            ..request
          }) {
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

  fn frustum_query_sub_primitives(
    &self,
    request: SceneModelFrustumSubPrimitiveQueryRequest,
  ) -> Option<()> {
    if let Some(internal) =
      self
        .internal
        .frustum_query_sub_primitives(SceneModelFrustumSubPrimitiveQueryRequest {
          results: request.results,
          ..request
        })
    {
      return Some(internal);
    }

    let view = self.get_view(
      request.idx,
      request.override_world_mat,
      request.ignore_pre_check,
    )?;

    for (i, m) in view.iter_mats().enumerate() {
      if let Some(positive) = self.internal.frustum_query(SceneModelFrustumQueryRequest {
        idx: view.source_model,
        override_world_mat: Some(&m),
        policy: request.policy,
        ignore_pre_check: true,
        frustum: request.frustum,
      }) {
        if positive {
          request.results.push(i as u32);
        }
      } else {
        return None;
      }
    }

    Some(())
  }
}
