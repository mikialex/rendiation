use crate::geometry::Geometry;
use crate::shading::Shading;
use rendiation_math::mat4::Mat4;
use rendiation_math_entity::*;
use std::rc::Rc;

pub struct RenderObject<Renderer> {
  pub index: usize,
  pub shading: Rc<dyn Shading<Renderer>>,
  pub geometry: Rc<dyn Geometry>,
  pub world_bounding_box: Box3,
  pub world_bounding_sphere: Sphere,
}

impl<Renderer> RenderObject<Renderer> {
  pub fn new(index: usize, shading: Rc<dyn Shading<Renderer>>, geometry: Rc<dyn Geometry>) -> Self {
    let world_bounding_box = *geometry.get_bounding_box();
    let world_bounding_sphere = *geometry.get_bounding_sphere();
    RenderObject {
      index,
      shading,
      geometry,
      world_bounding_box,
      world_bounding_sphere,
    }
  }

  pub fn update_world_bounding(&mut self, world_matrix: &Mat4<f32>) -> &mut Self {
    self.world_bounding_box = self
      .geometry
      .get_bounding_box()
      .clone()
      .apply_matrix(world_matrix);
    self.world_bounding_sphere = self
      .geometry
      .get_bounding_sphere()
      .clone()
      .apply_matrix(world_matrix);
    self
  }
}
