use crate::{environment::*, Vec3};
use crate::{light::*, Intersection, PossibleIntersection};
use crate::{model::*, RainRayGeometry};
use rendiation_algebra::*;
use rendiation_geometry::Ray3;

pub struct RainrayScene;

use sceno::SceneBackend;

impl SceneBackend for RainrayScene {
  type Drawable = Box<dyn RainrayModel>;

  type Material = Box<dyn RainrayMaterial>;

  type Mesh = Box<dyn RainRayGeometry>;

  type Background = Box<dyn Environment>;
}

pub type Scene = sceno::Scene<RainrayScene>;
pub type MeshHandle = sceno::Handle<Box<dyn RainRayGeometry>>;
pub type MaterialHandle = sceno::Handle<Box<dyn RainrayMaterial>>;

pub trait RainraySceneExt {
  fn get_min_dist_hit(&self, ray: Ray3) -> Option<(Intersection, &dyn RainrayModel)>;

  fn test_point_visible_to_point(&self, point_a: Vec3, point_b: Vec3) -> bool {
    let ray = Ray3::from_point_to_point(point_a, point_b);
    let distance = (point_a - point_b).length();

    if let Some(hit_result) = self.get_min_dist_hit(ray) {
      hit_result.0.distance > distance
    } else {
      true
    }
  }
}

impl RainraySceneExt for Scene {
  fn get_min_dist_hit(&self, ray: Ray3) -> Option<(Intersection, &dyn RainrayModel)> {
    let mut min_distance = std::f32::INFINITY;
    let mut result: Option<(Intersection, &dyn RainrayModel)> = None;
    for model in &self.drawables {
      if let PossibleIntersection(Some(mut intersection)) = model.intersect(&ray, &()) {
        if intersection.distance < min_distance {
          intersection.adjust_hit_position();
          min_distance = intersection.distance;
          result = Some((intersection, model.as_ref()))
        }
      }
    }
    result
  }
}

// pub struct Scene {
//   pub models: Vec<Box<dyn RainrayModel>>,
//   pub lights: Vec<Box<dyn Light>>,
//   pub env: Box<dyn Environment>,
// }

// impl Default for Scene {
//   fn default() -> Self {
//     Self {
//       models: Vec::new(),
//       lights: Vec::new(),
//       env: Box::new(SolidEnvironment::black()),
//     }
//   }
// }

// impl Scene {
//   pub fn get_min_dist_hit(&self, ray: Ray3) -> Option<(Intersection, &dyn RainrayModel)> {
//     let mut min_distance = std::f32::INFINITY;
//     let mut result: Option<(Intersection, &dyn RainrayModel)> = None;
//     for model in &self.models {
//       if let PossibleIntersection(Some(mut intersection)) = model.intersect(&ray, &()) {
//         if intersection.distance < min_distance {
//           intersection.adjust_hit_position();
//           min_distance = intersection.distance;
//           result = Some((intersection, model.as_ref()))
//         }
//       }
//     }
//     result
//   }

//   pub fn test_point_visible_to_point(&self, point_a: Vec3, point_b: Vec3) -> bool {
//     let ray = Ray3::from_point_to_point(point_a, point_b);
//     let distance = (point_a - point_b).length();

//     if let Some(hit_result) = self.get_min_dist_hit(ray) {
//       hit_result.0.distance > distance
//     } else {
//       true
//     }
//   }

//   pub fn environment(&mut self, env: impl Environment) -> &mut Self {
//     self.env = Box::new(env);
//     self
//   }

//   pub fn model(&mut self, model: impl RainrayModel) -> &mut Self {
//     self.models.push(Box::new(model));
//     self
//   }

//   pub fn light(&mut self, light: impl Light) -> &mut Self {
//     self.lights.push(Box::new(light));
//     self
//   }
// }
