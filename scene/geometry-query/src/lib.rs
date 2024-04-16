use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_mesh_core::{MeshBufferHitPoint, MeshBufferIntersectConfig};
use rendiation_scene_core::{Size, VirtualCollection};

pub struct SceneRayQuery<'a> {
  pub world_ray: Ray3,
  pub conf: &'a MeshBufferIntersectConfig,
  pub node_world: &'a dyn VirtualCollection<u32, Mat4<f32>>,
  pub node_visible: &'a dyn VirtualCollection<u32, bool>,
  pub camera_view_size: Size,
}

impl<'a> SceneRayQuery<'a> {
  pub fn query(&self) -> OptionalNearest<MeshBufferHitPoint> {
    todo!()
  }
}
