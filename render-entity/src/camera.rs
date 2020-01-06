use rendiation_math::*;
use rendiation_math_entity::*;

pub trait Camera {
  fn update_projection(&mut self);
  fn get_projection_matrix(&self) -> &Mat4<f32>;
  fn get_world_matrix(&self) -> &Mat4<f32>;
}

#[derive(Default)]
pub struct PerspectiveCamera{
  pub projection_matrix: Mat4<f32>,
  pub transform: Transformation,

  pub near: f32,
  pub far: f32,
  pub fov: f32,
  pub aspect: f32,
  pub zoom: f32,
}

impl PerspectiveCamera{
  pub fn new() -> Self {
    Self{
      projection_matrix: Mat4::<f32>::one(),
      transform: Transformation::new(),
    
      near: 1.,
      far: 100_000.,
      fov: 45.,
      aspect: 1.,
      zoom: 1.,
    }
  }
}


impl Camera for PerspectiveCamera{
  fn update_projection(&mut self){
    let top = self.near * (f32::pi_by_c180() * 0.5 * self.fov).tan() / self.zoom;
    let height = 2. * top;
    let width = self.aspect * height;
    let left = - 0.5 * width;
    self.projection_matrix.make_perspective(left, left + width, top, top - height, self.near, self.far);
  }

  fn get_projection_matrix(&self) -> &Mat4<f32>{
    &self.projection_matrix
  }

  fn get_world_matrix(&self) -> &Mat4<f32>{
    &self.transform.matrix
  }
}

#[derive(Default)]
pub struct AnyCamera {
  pub projection_matrix: Mat4<f32>,
  pub inverse_world_matrix: Mat4<f32>,
  // pub node: SceneNode
}

impl AnyCamera {
  pub fn new() -> Self {
    AnyCamera {
      projection_matrix: Mat4::one(),
      inverse_world_matrix: Mat4::one(),
      // node: SceneNode
    }
  }

  pub fn update_projection(&mut self, mat: &[f32]) {
    self.projection_matrix.a1 = mat[0];
    self.projection_matrix.a2 = mat[1];
    self.projection_matrix.a3 = mat[2];
    self.projection_matrix.a4 = mat[3];

    self.projection_matrix.b1 = mat[4];
    self.projection_matrix.b2 = mat[5];
    self.projection_matrix.b3 = mat[6];
    self.projection_matrix.b4 = mat[7];

    self.projection_matrix.c1 = mat[8];
    self.projection_matrix.c2 = mat[9];
    self.projection_matrix.c3 = mat[10];
    self.projection_matrix.c4 = mat[11];

    self.projection_matrix.d1 = mat[12];
    self.projection_matrix.d2 = mat[13];
    self.projection_matrix.d3 = mat[14];
    self.projection_matrix.d4 = mat[15];
  }

  pub fn update_inverse(&mut self, mat: &[f32]) {
    self.inverse_world_matrix.a1 = mat[0];
    self.inverse_world_matrix.a2 = mat[1];
    self.inverse_world_matrix.a3 = mat[2];
    self.inverse_world_matrix.a4 = mat[3];

    self.inverse_world_matrix.b1 = mat[4];
    self.inverse_world_matrix.b2 = mat[5];
    self.inverse_world_matrix.b3 = mat[6];
    self.inverse_world_matrix.b4 = mat[7];

    self.inverse_world_matrix.c1 = mat[8];
    self.inverse_world_matrix.c2 = mat[9];
    self.inverse_world_matrix.c3 = mat[10];
    self.inverse_world_matrix.c4 = mat[11];

    self.inverse_world_matrix.d1 = mat[12];
    self.inverse_world_matrix.d2 = mat[13];
    self.inverse_world_matrix.d3 = mat[14];
    self.inverse_world_matrix.d4 = mat[15];
  }
}
