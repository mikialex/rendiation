use database::*;
use reactive::*;
use rendiation_scene_core_next::*;
use rendiation_shader_api::*;
use rendiation_webgpu::*;
use rendiation_webgpu_reactive_utils::*;

mod material;
pub use material::*;

pub fn global_watch() -> DatabaseMutationWatch {
  todo!()
}
