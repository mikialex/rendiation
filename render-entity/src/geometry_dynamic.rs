use crate::geometry::Boundary3D;
use crate::geometry::Geometry;
use crate::buffer_data::BufferData;
use rendiation_math::vec3::Vec3;
use rendiation_math_entity::*;
use std::collections::HashMap;
use std::rc::Rc;

pub struct DynamicGeometry {
  pub bounding_box: Box3,
  pub bounding_sphere: Sphere,
  pub id: usize,
  pub index: Option<Rc<BufferData<u16>>>,

  pub attributes: HashMap<String, Rc<BufferData<f32>>>,
}

impl DynamicGeometry {
  pub fn new(
    id: usize,
    index: Option<Rc<BufferData<u16>>>,
    position: Rc<BufferData<f32>>,
  ) -> Result<DynamicGeometry, String> {
    if position.stride != 3 {
      Err(String::from("postion buffer is not stride of 3"))
    } else {
      let mut attributes = HashMap::new();
      attributes.insert(String::from("position"), position);
      let mut geo = DynamicGeometry {
        bounding_box: Box3::new(Vec3::new(1., 1., 1.), Vec3::new(1., 1., 1.)),
        bounding_sphere: Sphere::new(Vec3::new(1., 1., 1.), 1.),
        id,
        index,
        attributes,
      };
      geo.update_bounding();
      Ok(geo)
    }
  }
}

impl Geometry for DynamicGeometry {
  fn get_draw_count_all(&self) -> usize {
    if let Some(index) = &self.index {
      index.data.len()
    } else {
      self.attributes.get("position").unwrap().data.len() / 3
    }
  }

  fn is_index_draw(&self) -> bool {
    self.index.is_some()
  }

  fn get_index_attribute(&self) -> Option<&Rc<BufferData<u16>>> {
    self.index.as_ref()
  }
  fn get_attribute_by_name(&self, name: &str) -> Option<&Rc<BufferData<f32>>> {
    self.attributes.get(name)
  }
}

impl Boundary3D for DynamicGeometry {
  fn get_bounding_box(&self) -> &Box3 {
    &self.bounding_box
  }
  fn get_bounding_sphere(&self) -> &Sphere {
    &self.bounding_sphere
  }
  fn update_bounding(&mut self) {
    if let Some(position) = self.attributes.get("position") {
      self.bounding_box = Box3::make_from_position_buffer(&position.data);
      self.bounding_sphere =
        Sphere::make_from_position_buffer_with_box(&position.data, &self.bounding_box);
    }
  }
}
