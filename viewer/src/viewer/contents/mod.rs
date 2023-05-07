use crate::*;

mod fatline;
pub use fatline::*;
mod model_overrides;
pub use model_overrides::*;

pub fn register_viewer_extra_scene_features() {
  register_core_material_features::<SceneItemRef<FatLineMaterial>>();
  register_webgpu_material_features::<SceneItemRef<FatLineMaterial>>();
  register_core_mesh_features::<SceneItemRef<FatlineMesh>>();
  register_webgpu_mesh_features::<SceneItemRef<FatlineMesh>>();
}
