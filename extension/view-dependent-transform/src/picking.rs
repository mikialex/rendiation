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
  fn ray_query_nearest(
    &self,
    request: SceneModelRayNearestQueryRequest,
  ) -> Option<MeshBufferHitPoint<f64>> {
    let mat = self.get_mat(request.idx, request.override_world_mat);
    self
      .internal
      .ray_query_nearest(SceneModelRayNearestQueryRequest {
        override_world_mat: mat.as_ref(),
        ..request
      })
  }

  fn ray_query_all(&self, request: SceneModelRayAllQueryRequest) -> Option<()> {
    let mat = self.get_mat(request.idx, request.override_world_mat);
    self.internal.ray_query_all(SceneModelRayAllQueryRequest {
      override_world_mat: mat.as_ref(),
      ..request
    })
  }

  fn frustum_query(&self, request: SceneModelFrustumQueryRequest) -> Option<bool> {
    let mat = self.get_mat(request.idx, request.override_world_mat);
    self.internal.frustum_query(SceneModelFrustumQueryRequest {
      override_world_mat: mat.as_ref(),
      ..request
    })
  }

  fn frustum_query_sub_primitives(
    &self,
    request: SceneModelFrustumSubPrimitiveQueryRequest,
  ) -> Option<()> {
    let mat = self.get_mat(request.idx, request.override_world_mat);
    self
      .internal
      .frustum_query_sub_primitives(SceneModelFrustumSubPrimitiveQueryRequest {
        override_world_mat: mat.as_ref(),
        ..request
      })
  }
}
