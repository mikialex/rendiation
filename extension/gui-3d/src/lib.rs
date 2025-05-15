use database::*;
use fast_hash_collection::FastHashSet;
pub use hook::*;
use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_mesh_core::*;
use rendiation_mesh_generator::*;
use rendiation_scene_core::*;

mod hooks;
pub use hooks::*;

mod ty;
pub use ty::*;
mod model;
pub use model::*;
mod shape_helper;
pub use shape_helper::*;
mod interaction;
pub use interaction::*;
/// reexport
pub use rendiation_platform_event_input::*;

pub trait WidgetEnvAccess {
  fn get_world_mat(&self, sm: EntityHandle<SceneNodeEntity>) -> Option<Mat4<f32>>;
  fn get_camera_world_ray(&self) -> Ray3;
  /// xy -1 to 1
  fn get_normalized_canvas_position(&self) -> Vec2<f32>;
  fn get_camera_node(&self) -> EntityHandle<SceneNodeEntity>;
  fn get_camera_world_mat(&self) -> Mat4<f32> {
    self.get_world_mat(self.get_camera_node()).unwrap()
  }
  fn get_camera_perspective_proj(&self) -> PerspectiveProjection<f32>;
  fn get_camera_proj_mat(&self) -> Mat4<f32> {
    self
      .get_camera_perspective_proj()
      .compute_projection_mat(&OpenGLxNDC)
  }
  fn get_view_resolution(&self) -> Vec2<u32>;
}

pub fn register_gui3d_extension_data_model() {
  rendiation_wide_line::register_wide_line_data_model();
}
