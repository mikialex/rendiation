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
    idx: EntityHandle<SceneModelEntity>,
    override_world_mat: Option<&Mat4<f64>>,
    ctx: &SceneRayQuery,
  ) -> Option<MeshBufferHitPoint<f64>> {
    let mat = self.get_mat(idx, override_world_mat);
    self.internal.ray_query_nearest(idx, mat.as_ref(), ctx)
  }

  fn ray_query_all(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    override_world_mat: Option<&Mat4<f64>>,
    ctx: &SceneRayQuery,
    results: &mut Vec<MeshBufferHitPoint<f64>>,
    local_result_scratch: &mut Vec<MeshBufferHitPoint<f32>>,
  ) -> Option<()> {
    let mat = self.get_mat(idx, override_world_mat);
    self
      .internal
      .ray_query_all(idx, mat.as_ref(), ctx, results, local_result_scratch)
  }

  fn frustum_query(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    override_world_mat: Option<&Mat4<f64>>,
    frustum: &SceneFrustumQuery,
    policy: ObjectTestPolicy,
  ) -> Option<bool> {
    let mat = self.get_mat(idx, override_world_mat);
    self
      .internal
      .frustum_query(idx, mat.as_ref(), frustum, policy)
  }
}
