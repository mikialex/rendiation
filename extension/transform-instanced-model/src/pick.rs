use std::ops::ControlFlow;

use rendiation_scene_geometry_query::*;

use crate::*;

/// If the model is an instanced model, query the source model once per instance with the instance
/// matrix, hits and events are tagged with the instance index. Otherwise the request is forwarded
/// to the internal picker unchanged.
///
/// compare to [SceneModelPickerBaseImpl], we do not do instance buffer entire bbox check for ray.
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
    instance_model: EntityHandle<TransformInstancedModelEntity>,
    override_world_mat: Option<&Mat4<f64>>,
    ignore_pre_check: bool,
  ) -> Option<TransformPickView<'_>> {
    let node = self.util.pre_check(idx, ignore_pre_check)?;
    let instance_own_transform = self
      .util
      .resolve_mat_and_world_aabb(idx, node, override_world_mat)?
      .0;

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
  fn ray_query(&self, req: SceneModelRayQuery, v: &mut dyn RayHitVisitor) -> ControlFlow<()> {
    let Some(instance_model) = self.instance_model.get(req.idx) else {
      return self.internal.ray_query(req, v);
    };
    let Some(view) = self.get_view(
      req.idx,
      instance_model,
      req.override_world_mat,
      req.ignore_pre_check,
    ) else {
      return ControlFlow::Continue(());
    };

    for (i, m) in view.iter_mats().enumerate() {
      let source_req = SceneModelRayQuery {
        idx: view.source_model,
        override_world_mat: Some(&m),
        ctx: req.ctx,
        ignore_pre_check: true,
      };
      let mut tagged = RayHitInstanceTag {
        inner: &mut *v,
        model: req.idx,
        instance: i as u32,
      };
      self.internal.ray_query(source_req, &mut tagged)?;
    }
    ControlFlow::Continue(())
  }

  fn frustum_query(
    &self,
    req: SceneModelFrustumQuery,
    v: &mut dyn FrustumPrimitiveVisitor,
  ) -> ControlFlow<()> {
    let Some(instance_model) = self.instance_model.get(req.idx) else {
      return self.internal.frustum_query(req, v);
    };
    let Some(view) = self.get_view(
      req.idx,
      instance_model,
      req.override_world_mat,
      req.ignore_pre_check,
    ) else {
      return ControlFlow::Continue(());
    };

    for (i, m) in view.iter_mats().enumerate() {
      // if the whole instanced model is inside, every instance is
      let source_req = SceneModelFrustumQuery {
        idx: view.source_model,
        override_world_mat: Some(&m),
        frustum: req.frustum,
        policy: req.policy,
        known_inside: req.known_inside,
        ignore_pre_check: true,
      };
      let mut tagged = FrustumInstanceTag {
        inner: &mut *v,
        instance: i as u32,
      };
      self.internal.frustum_query(source_req, &mut tagged)?;
    }
    ControlFlow::Continue(())
  }
}
