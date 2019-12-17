
use crate::buffer_data::BufferData;
use rendiation_math_entity::*;
use std::rc::Rc;

pub trait Boundary3D {
  fn get_bounding_box(&self) -> &Box3;
  fn get_bounding_sphere(&self) -> &Sphere;
  fn update_bounding(&mut self);
}

pub trait Geometry: Boundary3D {
  fn get_draw_count_all(&self) -> usize;
  fn is_index_draw(&self) -> bool;
  fn get_index_attribute(&self) -> Option<&Rc<BufferData<u16>>>;
  fn get_attribute_by_name(&self, name: &str) -> Option<&Rc<BufferData<f32>>>;
}
