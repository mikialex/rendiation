use rendiation_mesh_gpu_system::*;

use crate::*;

impl MeshGPUInstance {
  pub fn get_bindless_mesh_handle(&self) -> Option<MeshSystemMeshHandle> {
    match self {
      _ => None,
    }
  }
}
// pub struct SinglePossibleBindless<'a> {
//   instance: &'a MeshGPUInstance,
// }
