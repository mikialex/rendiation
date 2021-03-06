use crate::*;
use rendiation_algebra::*;
use std::ops::{Deref, DerefMut};

impl<T, U> IntersectAble<Triangle<U>, Nearest<HitPoint3D<T>>, FaceSide> for Ray3<T>
where
  T: Scalar,
  U: Deref<Target = Vec3<T>> + Copy,
{
  #[allow(non_snake_case)]
  #[inline]
  fn intersect(&self, face: &Triangle<U>, side: &FaceSide) -> Nearest<HitPoint3D<T>> {
    let Triangle { a, b, c } = match side {
      FaceSide::Double | FaceSide::Front => face.map(|v| *v),
      FaceSide::Back => face.map(|v| *v).flip(),
    };

    let blackface_culling = match side {
      FaceSide::Double => false,
      FaceSide::Back | FaceSide::Front => true,
    };

    // from http://www.geometrictools.com/GTEngine/Include/Mathematics/GteIntrRay3Triangle3.h
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
    let mut sign = T::zero();

    if DdN > T::zero() {
      if blackface_culling {
        return Nearest::none();
      }
      sign = T::one();
    } else if DdN < T::zero() {
      sign = -T::one();
      DdN = -DdN;
    } else {
      return Nearest::none();
    }

    let _diff = self.origin - a;
    let DdQxE2 = sign * self.direction.dot(_diff.cross(_edge2));

    // b1 < 0, no intersection
    if DdQxE2 < T::zero() {
      return Nearest::none();
    }

    let DdE1xQ = sign * self.direction.dot(_edge1.cross(_diff));

    // b2 < 0, no intersection
    if DdE1xQ < T::zero() {
      return Nearest::none();
    }

    // b1+b2 > 1, no intersection
    if DdQxE2 + DdE1xQ > DdN {
      return Nearest::none();
    }

    // Line intersects triangle, check if ray does.
    let QdN = -sign * _diff.dot(_normal);

    // t < 0, no intersection
    if QdN < T::zero() {
      return Nearest::none();
    }

    // Ray3 intersects triangle.
    Nearest::some(self.at_into(QdN / DdN))
  }
}

impl<T, U> IntersectAble<LineSegment<U>, Nearest<HitPoint3D<T>>, T> for Ray3<T>
where
  T: Scalar,
  U: Deref<Target = Vec3<T>> + Copy,
{
  #[inline]
  fn intersect(&self, line: &LineSegment<U>, t: &T) -> Nearest<HitPoint3D<T>> {
    let (dist_sq, inter_ray, _) = self.distance_sq_to_segment(line.map(|v| *v));
    if dist_sq > *t * *t {
      return Nearest::none();
    }
    let distance = self.origin.distance(inter_ray);
    Nearest::some(HitPoint3D::new(inter_ray, distance))
  }
}

impl<T, U> IntersectAble<Point<U>, Nearest<HitPoint3D<T>>, T> for Ray3<T>
where
  T: Scalar,
  U: Deref<Target = Vec3<T>> + DerefMut + Copy,
{
  #[inline]
  fn intersect(&self, point: &Point<U>, t: &T) -> Nearest<HitPoint3D<T>> {
    let point = point.map(|v| *v).0;
    let dist_sq = self.distance_sq_to_point(point);
    if dist_sq > *t * *t {
      return Nearest::none();
    }
    let distance = self.origin.distance(point);
    Nearest::some(HitPoint3D::new(point, distance))
  }
}

intersect_reverse!(Box3, Nearest<HitPoint3D>, (), Ray3);
impl IntersectAble<Box3, Nearest<HitPoint3D>> for Ray3 {
  #[inline]
  fn intersect(&self, box3: &Box3, _: &()) -> Nearest<HitPoint3D> {
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
      return Nearest::none();
    }

    // These lines also handle the case where t_min or t_max is NaN

    if ty_min > t_min || t_min.is_nan() {
      t_min = ty_min
    };

    if ty_max < t_max || t_max.is_nan() {
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
      return Nearest::none();
    }

    if tz_min > t_min || t_min.is_nan() {
      t_min = tz_min;
    }

    if tz_max < t_max || t_max.is_nan() {
      t_max = tz_max;
    }

    //return point closest to the ray (positive side)

    if t_max < 0. {
      return Nearest::none();
    }

    Nearest::some(self.at_into(if t_min >= 0. { t_min } else { t_max }))
  }
}

intersect_reverse!(Box3, bool, (), Ray3);
impl IntersectAble<Box3, bool> for Ray3 {
  #[inline]
  fn intersect(&self, other: &Box3, p: &()) -> bool {
    IntersectAble::<Box3, Nearest<HitPoint3D>>::intersect(self, other, p).is_some()
  }
}

intersect_reverse!(Sphere, Nearest<HitPoint3D>, (), Ray3);
impl IntersectAble<Sphere, Nearest<HitPoint3D>> for Ray3 {
  #[inline]
  fn intersect(&self, sphere: &Sphere, _: &()) -> Nearest<HitPoint3D> {
    let oc = sphere.center - self.origin;
    let tca = oc.dot(self.direction);
    let d2 = oc.dot(oc) - tca * tca;
    let radius2 = sphere.radius * sphere.radius;

    if d2 > radius2 {
      return Nearest::none();
    };

    let thc = (radius2 - d2).sqrt();

    // t0 = first intersect point - entrance on front of sphere
    let t0 = tca - thc;

    // t1 = second intersect point - exit point on back of sphere
    let t1 = tca + thc;

    // test to see if both t0 and t1 are behind the ray - if so, return null
    if t0 < 0. && t1 < 0. {
      return Nearest::none();
    };

    // test to see if t0 is behind the ray:
    // if it is, the ray is inside the sphere, so return the second exit point scaled by t1,
    // in order to always return an intersect point that is in front of the ray.
    if t0 < 0. {
      return Nearest::some(self.at_into(t1));
    };

    // else t0 is in front of the ray, so return the first collision point scaled by t0
    Nearest::some(self.at_into(t0))
  }
}

intersect_reverse!(Sphere, bool, (), Ray3);
impl IntersectAble<Sphere, bool> for Ray3 {
  #[inline]
  fn intersect(&self, other: &Sphere, p: &()) -> bool {
    IntersectAble::<Sphere, Nearest<HitPoint3D>>::intersect(self, other, p).is_some()
  }
}

impl IntersectAble<Plane, Nearest<HitPoint3D>> for Ray3 {
  #[inline]
  fn intersect(&self, plane: &Plane, _: &()) -> Nearest<HitPoint3D> {
    self
      .distance_to_plane(plane)
      .map(|distance| self.at_into(distance))
      .into()
  }
}
