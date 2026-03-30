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
  pub projection: Mat4<f32>,
  pub projection_inv: Mat4<f32>,
  pub proj_source: Option<CommonProjection>,
  pub camera_world_mat: Mat4<f64>,
}

impl ViewportPointerCtx {
  pub fn create_ratio_cal(&self) -> Box<dyn Fn(f32, f32) -> f32> {
    if let Some(proj_source) = self.proj_source {
      match proj_source {
        CommonProjection::Perspective(p) => {
          Box::new(move |d, h| p.pixels_per_unit(d, h)) as Box<dyn Fn(f32, f32) -> f32>
        }
        CommonProjection::Orth(p) => Box::new(move |d, h| p.pixels_per_unit(d, h)),
      }
    } else {
      let projection = self.projection;
      let projection_inv = self.projection_inv;
      Box::new(move |d, h| projection.pixels_per_unit(projection_inv, d, h))
    }
  }
}

#[derive(Copy, Clone)]
pub enum CommonProjection {
  Perspective(PerspectiveProjection<f32>),
  Orth(OrthographicProjection<f32>),
}

impl CommonProjection {
  pub fn compute_projection_mat(&self, mapper: &dyn NDCSpaceMapper<f32>) -> Mat4<f32> {
    match self {
      CommonProjection::Perspective(p) => p.compute_projection_mat(mapper),
      CommonProjection::Orth(p) => p.compute_projection_mat(mapper),
    }
  }
}

pub trait WidgetEnvAccess {
  fn get_world_mat(&self, sm: EntityHandle<SceneNodeEntity>) -> Option<Mat4<f64>>;

  /// return None, if current mouse is not in any viewport
  fn get_viewport_pointer_ctx(&self) -> Option<&ViewportPointerCtx>;
}

pub fn register_gui3d_extension_data_model(sparse: bool) {
  rendiation_wide_line::register_wide_line_data_model(sparse);
}
