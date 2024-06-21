use database::*;
use reactive::*;
use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_mesh_core::{MeshBufferHitPoint, MeshBufferIntersectConfig};
use rendiation_scene_core::*;
use rendiation_texture_core::Size;

pub struct SceneRayQuery {
  pub world_ray: Ray3,
  pub conf: MeshBufferIntersectConfig,
  pub node_world: Box<dyn DynVirtualCollection<u32, Mat4<f32>>>,
  pub node_visible: Box<dyn DynVirtualCollection<u32, bool>>,
  pub model_lookup:
    Box<dyn DynVirtualMultiCollection<EntityHandle<SceneEntity>, EntityHandle<SceneModelEntity>>>,
  pub camera_view_size: Size,

  pub scene_model_picker: Vec<Box<dyn SceneModelPicker>>,
}

pub trait SceneModelPicker {
  fn query(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
  ) -> Option<MeshBufferHitPoint>;
}

impl SceneModelPicker for Vec<Box<dyn SceneModelPicker>> {
  fn query(
    &self,
    idx: EntityHandle<SceneModelEntity>,
    ctx: &SceneRayQuery,
  ) -> Option<MeshBufferHitPoint> {
    for provider in self {
      if let Some(hit) = provider.query(idx, ctx) {
        return Some(hit);
      }
    }
    None
  }
}

impl SceneRayQuery {
  pub fn query(&self, scene: EntityHandle<SceneEntity>) -> OptionalNearest<MeshBufferHitPoint> {
    let mut nearest = OptionalNearest::none();
    for idx in self.model_lookup.access_multi_value(&scene) {
      if let Some(hit) = self.scene_model_picker.query(idx, self) {
        nearest.refresh(hit);
      }
    }

    nearest
  }
}
