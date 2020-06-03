use crate::math::*;

pub static MAX_RAY_HIT_DISTANCE: f32 = 1000.0;
pub static EPS: f32 = 0.00001;

// pub type RayIntersectAble = dyn IntersectAble<Ray3, Option<Intersection>>;

pub trait RayIntersectAble {
  fn intersect(&self, ray: &Ray3) -> Option<Intersection>;
}

pub struct Intersection {
  pub distance: f32,
  pub hit_position: Vec3,
  pub hit_normal: Vec3,
}

impl RayIntersectAble for Sphere {
  fn intersect(&self, ray: &Ray3) -> Option<Intersection> {
    let voc = self.center - ray.origin; // Vector from the origin to the sphere center
    let voc_len_sqr = voc.length2(); // The length squared of voc
    let vod_len = voc.dot(ray.direction); // The length of the projected vector voc into the ray direction

    let a_sqr = voc_len_sqr - (vod_len * vod_len); // The length squared of the line between c and the ray
    let radius_square = self.radius * self.radius; // Radius squared
                                                   // println!("{}", a_sqr);
    if a_sqr <= radius_square + EPS {
      let b = (radius_square - a_sqr).sqrt(); // the distance between o and the intersection with the sphere

      let distance = if vod_len - b < 0.0 {
        vod_len + b
      } else {
        vod_len - b
      };

      if distance > EPS {
        if distance > MAX_RAY_HIT_DISTANCE {
          return None; // too far
        }
        let hit_position = ray.at(distance);
        let hit_normal = (hit_position - self.center).normalize();
        Some(Intersection {
          distance,
          hit_normal,
          hit_position,
        })
      } else {
        None // opposite direction
      }
    } else {
      None // not intersect
    }
  }
}
