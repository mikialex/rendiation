use bytemuck::cast_slice;
use database::*;
use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_mesh_core::*;
use rendiation_scene_core::*;
use rendiation_texture_core::Size;

mod model;
pub use model::*;

mod scene_model;
pub use scene_model::*;

mod scene;
pub use scene::*;

pub struct SceneRayQuery {
  pub world_ray: Ray3<f64>,
  pub camera_view_size_in_logic_pixel: Size,
  pub camera_proj: Box<dyn Projection<f32>>,
  pub camera_world: Mat4<f64>,
}

impl SceneRayQuery {
  pub fn compute_local_tolerance(
    &self,
    tolerance: IntersectTolerance,
    target_world_mat: Mat4<f64>,
    camera_world_mat: Mat4<f64>,
    target_object_center_in_world: Vec3<f64>,
  ) -> f32 {
    let target_scale = target_world_mat.max_scale();
    // todo, should we considering camera scale??
    let mut local_tolerance = tolerance.value / target_scale as f32;

    if let ToleranceType::ScreenSpace = tolerance.ty {
      let camera_to_target = target_object_center_in_world - self.world_ray.origin;
      let projected_distance = camera_to_target.dot(camera_world_mat.forward().reverse());
      let pixel_per_unit = self.camera_proj.pixels_per_unit(
        projected_distance as f32,
        self.camera_view_size_in_logic_pixel.height_usize() as f32,
      );
      local_tolerance /= pixel_per_unit;
    }

    local_tolerance
  }
}
