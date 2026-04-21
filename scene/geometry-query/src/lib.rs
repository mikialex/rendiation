use bytemuck::cast_slice;
use database::*;
use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_mesh_core::*;
pub use rendiation_mesh_core::{IntersectTolerance, MeshBufferHitPoint, ToleranceType};
use rendiation_scene_core::*;
use rendiation_texture_core::Size;

mod model;
pub use model::*;

mod scene_model;
pub use scene_model::*;

mod iter;
pub use iter::*;

declare_component!(SceneModelSelectable, SceneModelEntity, bool, true);
pub fn register_selectable_data_model() {
  global_entity_of::<SceneModelEntity>().declare_component::<SceneModelSelectable>();
}

pub struct SceneRayQuery {
  pub world_ray: Ray3<f64>,
  pub camera_ctx: CameraQueryCtx,
}

pub struct SceneFrustumQuery {
  pub world_frustum: Frustum<f64>,
  pub camera_ctx: CameraQueryCtx,
}

pub struct CameraQueryCtx {
  pub camera_view_size_in_logic_pixel: Size,
  pub pixels_per_unit_calc: Box<dyn Fn(f32, f32) -> f32>,
  pub camera_world: Mat4<f64>,
  pub camera_vp: Mat4<f64>,
}

impl CameraQueryCtx {
  pub fn compute_local_tolerance(
    &self,
    tolerance: IntersectTolerance,
    target_world_mat_max_scale: f64,
    target_object_center_in_world: Vec3<f64>,
  ) -> f32 {
    // todo, should we considering camera scale??
    let mut local_tolerance = tolerance.value / target_world_mat_max_scale as f32;

    if let ToleranceType::ScreenSpace = tolerance.ty {
      let camera_to_target = target_object_center_in_world - self.camera_world.position();
      let projected_distance = camera_to_target.dot(self.camera_world.forward().reverse());
      let pixel_per_unit = (self.pixels_per_unit_calc)(
        projected_distance as f32,
        self.camera_view_size_in_logic_pixel.height_usize() as f32,
      );
      local_tolerance /= pixel_per_unit;
    }

    local_tolerance
  }
}
