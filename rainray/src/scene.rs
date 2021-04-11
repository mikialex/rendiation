use crate::{background::*, RainrayMaterial, Vec3};
use crate::{light::*, Intersection, PossibleIntersection};
use crate::{model::*, RainRayGeometry};
use rendiation_algebra::*;
use rendiation_geometry::Ray3;
use sceno::SceneBackend;

pub struct RainrayScene;

impl SceneBackend for RainrayScene {
  type Model = Box<dyn RainrayModel>;
  type Material = Box<dyn RainrayMaterial>;
  type Mesh = Box<dyn RainRayGeometry>;
  type Background = Box<dyn Background>;
  type Light = Box<dyn Light>;
}

pub type Scene = sceno::Scene<RainrayScene>;
pub type MeshHandle = sceno::MeshHandle<RainrayScene>;
pub type MaterialHandle = sceno::MaterialHandle<RainrayScene>;

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
    for (_, model) in &self.models {
      if let PossibleIntersection(Some(mut intersection)) = model.intersect(&ray, self) {
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
