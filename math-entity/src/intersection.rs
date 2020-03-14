use crate::box3::Box3;
use crate::ray::Ray;
use crate::sphere::Sphere;
use rendiation_math::Vec3;

pub struct NearestPoint3D(pub Vec3<f32>);

pub trait IntersectAble<T> {
  type IntersectResult;
  fn intersect(&self, other: &T) -> Option<Self::IntersectResult>;
  fn if_intersect(&self, other: &T) -> bool {
    // ok, maybe it will be optimized by compiler;
    self.intersect(other).is_some()
  }
}

impl IntersectAble<Box3> for Ray {
  type IntersectResult = NearestPoint3D;

  fn intersect(&self, box3: &Box3) -> Option<NearestPoint3D> {
    #[allow(unused_assignments)]
    let (mut t_max, mut t_min, mut ty_min, mut ty_max, mut tz_min, mut tz_max) =
      (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);

    let inv_dir_x = 1. / self.direction.x;
    let inv_dir_y = 1. / self.direction.y;
    let inv_dir_z = 1. / self.direction.z;

    let origin = self.origin;
    if inv_dir_x >= 0. {
      t_min = (box3.min.x - origin.x) * inv_dir_x;
      t_max = (box3.max.x - origin.x) * inv_dir_x;
    } else {
      t_min = (box3.max.x - origin.x) * inv_dir_x;
      t_max = (box3.min.x - origin.x) * inv_dir_x;
    }

    if inv_dir_y >= 0. {
      ty_min = (box3.min.y - origin.y) * inv_dir_y;
      ty_max = (box3.max.y - origin.y) * inv_dir_y;
    } else {
      ty_min = (box3.max.y - origin.y) * inv_dir_y;
      ty_max = (box3.min.y - origin.y) * inv_dir_y;
    }

    if (t_min > ty_max) || (ty_min > t_max) {
      return None;
    }

    // These lines also handle the case where t_min or t_max is NaN
    // (result of 0 * Infinity). x !== x returns true if x is NaN

    if ty_min > t_min || t_min != t_min {
      t_min = ty_min
    };

    if ty_max < t_max || t_max != t_max {
      t_max = ty_max
    };

    if inv_dir_z >= 0. {
      tz_min = (box3.min.z - origin.z) * inv_dir_z;
      tz_max = (box3.max.z - origin.z) * inv_dir_z;
    } else {
      tz_min = (box3.max.z - origin.z) * inv_dir_z;
      tz_max = (box3.min.z - origin.z) * inv_dir_z;
    }

    if (t_min > tz_max) || (tz_min > t_max) {
      return None;
    }

    if tz_min > t_min || t_min != t_min {
      t_min = tz_min;
    }

    if tz_max < t_max || t_max != t_max {
      t_max = tz_max;
    }

    //return point closest to the ray (positive side)

    if t_max < 0. {
      return None;
    }

    Some(NearestPoint3D(self.at(if t_min >= 0. { t_min } else { t_max })))
  }
}

impl IntersectAble<Sphere> for Ray {
  
  type IntersectResult = NearestPoint3D;

  fn intersect(&self, sphere: &Sphere) -> Option<NearestPoint3D> {
    let oc = sphere.center - self.origin;
    let tca = oc.dot(self.direction);
    let d2 = oc.dot(oc) - tca * tca;
    let radius2 = sphere.radius * sphere.radius;

    if d2 > radius2 {
      return None;
    };

    let thc = (radius2 - d2).sqrt();

    // t0 = first intersect point - entrance on front of sphere
    let t0 = tca - thc;

    // t1 = second intersect point - exit point on back of sphere
    let t1 = tca + thc;

    // test to see if both t0 and t1 are behind the ray - if so, return null
    if t0 < 0. && t1 < 0. {
      return None;
    };

    // test to see if t0 is behind the ray:
    // if it is, the ray is inside the sphere, so return the second exit point scaled by t1,
    // in order to always return an intersect point that is in front of the ray.
    if t0 < 0. {
      return Some(NearestPoint3D(self.at(t1)));
    };

    // else t0 is in front of the ray, so return the first collision point scaled by t0
    Some(NearestPoint3D(self.at(t0)))
  }
}
