use crate::math::*;

pub struct Camera {
  pub eye_position: Vec3,
  pub up_direction: Vec3,

  pub film_width: f32,
  pub film_height: f32,
  pub film_center: Vec3,
}

impl Camera {
  pub fn new() -> Camera {
    Camera {
      eye_position: Vec3::new(0., 0., 7.),
      up_direction: Vec3::new(0., 1., 0.),
      film_width: 4.,
      film_height: 4.,
      film_center: Vec3::new(0., 0., 3.),
    }
  }

  pub fn get_pixel_world_position(&self, x_ratio: f32, y_ratio: f32) -> Vec3 {
    let clamped_x_ratio = x_ratio.max(0.0).min(1.0) - 0.5;
    let clamped_y_ratio = y_ratio.max(0.0).min(1.0) - 0.5;
    let center_direction = self.film_center - self.eye_position;
    let x_axis = self.up_direction.cross(center_direction).normalize();
    let y_axis = x_axis.cross(center_direction).normalize();
    let mut film_position = self.film_center + x_axis * self.film_width * clamped_x_ratio;
    film_position = film_position + y_axis * self.film_height * clamped_y_ratio;
    return film_position;
  }

  pub fn generate_pixel_ray(&self, x_ratio: f32, y_ratio: f32) -> Ray {
    let mut ray = Ray::from_point_to_point(
      self.eye_position,
      self.get_pixel_world_position(x_ratio, y_ratio),
    );
    ray.direction = ray.direction.normalize();
    ray
  }
}
