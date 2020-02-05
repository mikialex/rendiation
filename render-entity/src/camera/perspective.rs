use super::Camera;
use crate::transformed_object::TransformedObject;
use rendiation_math::*;
use rendiation_math_entity::*;

#[derive(Default)]
pub struct PerspectiveCamera {
  pub projection_matrix: Mat4<f32>,
  pub transform: Transformation,

  pub near: f32,
  pub far: f32,
  pub fov: f32,
  pub aspect: f32,
  pub zoom: f32,
}

impl PerspectiveCamera {
  pub fn new() -> Self {
    Self {
      projection_matrix: Mat4::<f32>::one(),
      transform: Transformation::new(),
      near: 1.,
      far: 100_000.,
      fov: 90.,
      aspect: 1.,
      zoom: 1.,
    }
  }

  pub fn create_screen_ray(&self, screen_x_ratio: f32, screen_y_ratio: f32) -> Ray {
    let position = self.get_transform().matrix.position();
    let target = Vec3::new(screen_x_ratio * 2. - 1., screen_y_ratio * 2. - 1., 0.5);
    // let un_projection = self.get_projection_matrix().inverse() * self.get_transform().matrix;
    let un_projected1 = target * self.get_projection_matrix().inverse();
    let un_projected = target * self.get_projection_matrix().inverse() * self.get_transform().matrix;
    let direction = (un_projected - position).normalize();
    println!("");
    println!("position {}", position);
    // println!("debug {}", Vec3::new(0.,0.,0.) * self.get_projection_matrix().inverse());
    println!("target {}", target);
    println!("un_projected {}", un_projected);
    println!("dir {}", direction);
    Ray::new(position, direction)
  }
}

impl TransformedObject for PerspectiveCamera {
  fn get_transform(&self) -> &Transformation {
    &self.transform
  }

  fn get_transform_mut(&mut self) -> &mut Transformation {
    &mut self.transform
  }
}

impl Camera for PerspectiveCamera {
  fn update_projection(&mut self) {
    self.projection_matrix = Mat4::perspective_fov_rh(self.fov, self.aspect, self.near, self.far);
  }

  fn get_projection_matrix(&self) -> &Mat4<f32> {
    &self.projection_matrix
  }

  fn resize(&mut self, size: (f32, f32)) {
    self.aspect = size.0 / size.1;
    self.update_projection();
  }
}
