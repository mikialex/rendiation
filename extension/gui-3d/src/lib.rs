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
mod view_dependent_node;
/// reexport
pub use rendiation_platform_event_input::*;
pub use rendiation_view_override_model::*;
pub use view_dependent_node::*;

#[derive(Clone)]
pub struct ViewportPointerCtx {
  pub world_ray: Ray3<f64>,
  pub viewport_idx: usize,
  pub viewport_id: u64,
  pub view_logical_pixel_size: Vec2<u32>,
  /// xy -1 to 1
  pub normalized_position: Vec2<f32>,
  pub perspective_proj: PerspectiveProjection<f32>,
  pub camera_world_mat: Mat4<f64>,
}

impl ViewportPointerCtx {
  pub fn camera_projection_mat(&self) -> Mat4<f32> {
    self.perspective_proj.compute_projection_mat(&OpenGLxNDC)
  }
}

pub trait WidgetEnvAccess {
  fn get_world_mat(&self, sm: EntityHandle<SceneNodeEntity>) -> Option<Mat4<f64>>;

  /// return None, if current mouse is not in any viewport
  fn get_viewport_pointer_ctx(&self) -> Option<&ViewportPointerCtx>;
}

pub fn register_gui3d_extension_data_model() {
  rendiation_wide_line::register_wide_line_data_model();
}
