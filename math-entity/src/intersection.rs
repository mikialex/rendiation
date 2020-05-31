use crate::box3::Box3;
use crate::face3::Face3;
use crate::line3::Line3;
use crate::ray::Ray;
use crate::sphere::Sphere;
use rendiation_math::Vec3;

macro_rules! intersect_reverse {
  ($self_item: ty, $result:ty, $param:ty, $target:ty) => {
    impl IntersectAble<$target, $result, $param> for $self_item {
      fn intersect(&self, other: &$target, p: &$param) -> $result {
        IntersectAble::<$self_item, $result, $param>::intersect(other, self, p)
      }
    }
  };
}

pub struct NearestPoint3D(pub Option<Vec3<f32>>);
pub struct IntersectionList(pub Vec<Vec3<f32>>);

pub trait IntersectAble<Target, Result, Parameter = ()> {
  fn intersect(&self, other: &Target, param: &Parameter) -> Result;
}

// impl<T, Target, Result, Parameter> IntersectAble<Target, Result, Parameter> for T
//   where Target: IntersectAble<T, Result, Parameter>
// {
//   fn intersect(&self, other: &Target, param: &Parameter) -> Result{
//     IntersectAble::<T, Result, Parameter>::intersect(other, self, param)
//   }
// }

impl IntersectAble<Face3, NearestPoint3D> for Ray {
  #[allow(non_snake_case)]
  fn intersect(&self, face: &Face3, _: &()) -> NearestPoint3D {
    // Compute the offset origin, edges, and normal.

    // from http://www.geometrictools.com/GTEngine/Include/Mathematics/GteIntrRay3Triangle3.h
    let Face3 { a, b, c } = *face;
    let blackfaceCulling = false;
    let _edge1 = b - a;
    let _edge2 = c - a;
    let _normal = _edge1.cross(_edge2);

    // Solve Q + t*D = b1*E1 + b2*E2 (Q = kDiff, D = ray direction,
    // E1 = kEdge1, E2 = kEdge2, N = Cross(E1,E2)) by
    //   |Dot(D,N)|*b1 = sign(Dot(D,N))*Dot(D,Cross(Q,E2))
    //   |Dot(D,N)|*b2 = sign(Dot(D,N))*Dot(D,Cross(E1,Q))
    //   |Dot(D,N)|*t = -sign(Dot(D,N))*Dot(Q,N)
    let mut DdN = self.direction.dot(_normal);
    #[allow(unused_assignments)]
    let mut sign: f32 = 0.;

    if DdN > 0. {
      if blackfaceCulling {
        return NearestPoint3D(None);
      }
      sign = 1.;
    } else if DdN < 0.0 {
      sign = -1.;
      DdN = -DdN;
    } else {
      return NearestPoint3D(None);
    }

    let _diff = self.origin - a;
    let DdQxE2 = sign * self.direction.dot(_diff.cross(_edge2));

    // b1 < 0, no intersection
    if DdQxE2 < 0. {
      return NearestPoint3D(None);
    }

    let DdE1xQ = sign * self.direction.dot(_edge1.cross(_diff));

    // b2 < 0, no intersection
    if DdE1xQ < 0. {
      return NearestPoint3D(None);
    }

    // b1+b2 > 1, no intersection
    if DdQxE2 + DdE1xQ > DdN {
      return NearestPoint3D(None);
    }

    // Line intersects triangle, check if ray does.
    let QdN = -sign * _diff.dot(_normal);

    // t < 0, no intersection
    if QdN < 0. {
      return NearestPoint3D(None);
    }

    // Ray intersects triangle.
    return NearestPoint3D(Some(self.at(QdN / DdN)));
  }
}

impl IntersectAble<Ray, NearestPoint3D> for Face3 {
  fn intersect(&self, ray: &Ray, p: &()) -> NearestPoint3D {
    IntersectAble::<Face3, NearestPoint3D>::intersect(ray, self, p)
  }
}

impl IntersectAble<Ray, NearestPoint3D> for Line3 {
  fn intersect(&self, ray: &Ray, _: &()) -> NearestPoint3D {
    todo!()
  }
}

impl IntersectAble<Box3, NearestPoint3D> for Ray {
  fn intersect(&self, box3: &Box3, _: &()) -> NearestPoint3D {
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
      return NearestPoint3D(None);
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
      return NearestPoint3D(None);
    }

    if tz_min > t_min || t_min != t_min {
      t_min = tz_min;
    }

    if tz_max < t_max || t_max != t_max {
      t_max = tz_max;
    }

    //return point closest to the ray (positive side)

    if t_max < 0. {
      return NearestPoint3D(None);
    }

    NearestPoint3D(Some(self.at(if t_min >= 0. { t_min } else { t_max })))
  }
}

intersect_reverse!(Box3, bool, (), Ray);
impl IntersectAble<Box3, bool> for Ray {
  fn intersect(&self, other: &Box3, p: &()) -> bool {
    IntersectAble::<Box3, NearestPoint3D>::intersect(self, other, p)
      .0
      .is_some()
  }
}

impl IntersectAble<Sphere, NearestPoint3D> for Ray {
  fn intersect(&self, sphere: &Sphere, _: &()) -> NearestPoint3D {
    let oc = sphere.center - self.origin;
    let tca = oc.dot(self.direction);
    let d2 = oc.dot(oc) - tca * tca;
    let radius2 = sphere.radius * sphere.radius;

    if d2 > radius2 {
      return NearestPoint3D(None);
    };

    let thc = (radius2 - d2).sqrt();

    // t0 = first intersect point - entrance on front of sphere
    let t0 = tca - thc;

    // t1 = second intersect point - exit point on back of sphere
    let t1 = tca + thc;

    // test to see if both t0 and t1 are behind the ray - if so, return null
    if t0 < 0. && t1 < 0. {
      return NearestPoint3D(None);
    };

    // test to see if t0 is behind the ray:
    // if it is, the ray is inside the sphere, so return the second exit point scaled by t1,
    // in order to always return an intersect point that is in front of the ray.
    if t0 < 0. {
      return NearestPoint3D(Some(self.at(t1)));
    };

    // else t0 is in front of the ray, so return the first collision point scaled by t0
    NearestPoint3D(Some(self.at(t0)))
  }
}

impl IntersectAble<Sphere, bool> for Ray {
  fn intersect(&self, other: &Sphere, p: &()) -> bool {
    IntersectAble::<Sphere, NearestPoint3D>::intersect(self, other, p).0.is_some()
  }
}

impl IntersectAble<Sphere, Option<IntersectionList>> for Ray {
  fn intersect(&self, sphere: &Sphere, _: &()) -> Option<IntersectionList> {
    todo!();
  }
}
