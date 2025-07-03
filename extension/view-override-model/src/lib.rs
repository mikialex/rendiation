use rendiation_algebra::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BillBoard {
  /// define what the front direction is (in object space)
  ///
  /// the front_direction will always lookat the view direction
  pub front_direction: Vec3<f32>,
}

impl BillBoard {
  pub fn override_mat(&self, world_matrix: Mat4<f64>, camera_position: Vec3<f64>) -> Mat4<f64> {
    let scale = world_matrix.get_scale();
    let scale = Mat4::scale(scale);
    let position = world_matrix.position();
    let position_m = Mat4::translate(position);

    let correction = Mat4::lookat(
      Vec3::new(0., 0., 0.),
      self.front_direction.into_f64(),
      Vec3::new(0., 1., 0.),
    );

    let rotation = Mat4::lookat(position, camera_position, Vec3::new(0., 1., 0.));

    // there must be cheaper ways
    position_m * rotation * correction * scale
  }
}

impl Default for BillBoard {
  fn default() -> Self {
    Self {
      front_direction: Vec3::new(0., 0., 1.),
    }
  }
}

/// the position by default will choose by the node's world matrix;
///
/// but in sometimes, we need use another position for position
/// to keep consistent dynamic scale behavior among the group of scene node hierarchy.
/// in this case, we can use this override_position and update this position manually.
pub enum ViewAutoScalablePositionOverride {
  Fixed(Vec3<f32>),
  SyncNode(u32),
}

#[derive(Debug, Clone, Copy)]
pub struct ViewAutoScalable {
  pub independent_scale_factor: f32,
}

impl ViewAutoScalable {
  pub fn compute_scale(
    &self,
    override_position: Vec3<f64>,
    camera_world: Mat4<f64>,
    view_height_in_pixel: f32,
    camera_proj: impl Projection<f32>,
  ) -> f32 {
    let camera_position = camera_world.position();
    let camera_forward = camera_world.forward().reverse().normalize();
    let camera_to_target = override_position - camera_position;

    let projected_distance = camera_to_target.dot(camera_forward);

    self.independent_scale_factor
      / camera_proj.pixels_per_unit(projected_distance as f32, view_height_in_pixel)
  }

  pub fn override_mat(
    &self,
    world_matrix: Mat4<f64>,
    override_position: Vec3<f64>,
    camera_world: Mat4<f64>,
    view_height_in_pixel: f32,
    camera_proj: impl Projection<f32>,
  ) -> Mat4<f64> {
    let world_translation = Mat4::translate(override_position);

    let scale = self.compute_scale(
      override_position,
      camera_world,
      view_height_in_pixel,
      camera_proj,
    );

    world_translation // move back to position
      * Mat4::scale(Vec3::splat(scale as f64)) // apply new scale
      * world_translation.inverse_or_identity() // move back to zero
      * world_matrix // original
  }
}

pub struct InverseWorld;

impl InverseWorld {
  pub fn override_mat(&self, world_matrix: Mat4<f64>) -> Mat4<f64> {
    world_matrix.inverse_or_identity()
  }
}
