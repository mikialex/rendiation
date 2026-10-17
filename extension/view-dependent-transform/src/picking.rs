use std::ops::ControlFlow;

use rendiation_scene_geometry_query::*;

use crate::*;

pub struct SceneModelPickerWithViewDep<T> {
  pub internal: T,
  pub view_mats: BoxedDynQuery<ViewSceneModelKey, Mat4<f64>>,
  pub active_view: Option<u64>,
}

impl<T> SceneModelPickerWithViewDep<T> {
  pub fn set_active_view(&mut self, view_id: Option<u64>) {
    self.active_view = view_id;
  }
  fn get_mat(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    override_world_mat: Option<&Mat4<f64>>,
  ) -> Option<Mat4<f64>> {
    if let Some(mat) = override_world_mat.copied() {
      Some(mat)
    } else {
      if let Some(active_view) = self.active_view {
        self.view_mats.access(&(active_view, idx.into_raw()))
      } else {
        None
      }
    }
  }
}

impl<T: SceneModelPicker> SceneModelPicker for SceneModelPickerWithViewDep<T> {
  fn ray_query(&self, req: SceneModelRayQuery, v: &mut dyn RayHitVisitor) -> ControlFlow<()> {
    let mat = self.get_mat(req.idx, req.override_world_mat);
    self.internal.ray_query(
      SceneModelRayQuery {
        override_world_mat: mat.as_ref(),
        ..req
      },
      v,
    )
  }

  fn frustum_query(
    &self,
    req: SceneModelFrustumQuery,
    v: &mut dyn FrustumPrimitiveVisitor,
  ) -> ControlFlow<()> {
    let mat = self.get_mat(req.idx, req.override_world_mat);
    self.internal.frustum_query(
      SceneModelFrustumQuery {
        override_world_mat: mat.as_ref(),
        ..req
      },
      v,
    )
  }
}
