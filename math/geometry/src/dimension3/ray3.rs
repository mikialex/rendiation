use crate::*;

pub type Ray3<T = f32> = HyperRay<T, Vec3<T>>;

impl<T: Scalar> SpaceEntity<T, 3> for Ray3<T> {
  type Matrix = Mat4<T>;
  #[inline]
  fn apply_matrix(&mut self, mat: Self::Matrix) -> &mut Self {
    let origin = mat * self.origin;
    let direction = self.direction.transform_direction(mat);
    *self = Self::new(origin, direction);
    self
  }
}

impl<T: Scalar> Ray3<T> {
  pub fn from_origin_to_target(origin: Vec3<T>, target: Vec3<T>) -> Self {
    Ray3::new(origin, (target - origin).into_normalized())
  }

  // this distance to can not expressed by DistanceSquareTo trait
  pub fn distance_sq_to_point(&self, point: Vec3<T>) -> T {
    let oc = point - self.origin;
    let tca = oc.dot(self.direction);
    oc.dot(oc) - tca * tca
  }

  // this distance to can not expressed by DistanceTo trait
  pub fn distance_to_plane(&self, plane: &Plane<T>) -> Option<T> {
    let denominator = plane.normal.dot(self.direction);

    if denominator == T::zero() {
      // line is coplanar, return origin
      if plane.distance_to(&self.origin) == T::zero() {
        return T::zero().into();
      }

      return None;
    }

    let t = -(self.origin.dot(plane.normal) + plane.constant) / denominator;

    // Return if the ray never intersects the plane
    if t >= T::zero() {
      t.into()
    } else {
      None
    }
  }

  // this distance to can not expressed by DistanceSquareTo trait
  pub fn distance_sq_to_segment(&self, line: LineSegment3D<T>) -> (T, Vec3<T>, Vec3<T>) {
    // (distance_sq_to_segment, optionalPointOnRay, optionalPointOnSegment)
    let v0 = line.start;
    let v1 = line.end;

    // from http://www.geometrictools.com/GTEngine/Include/Mathematics/GteDistRaySegment.h
    // It returns the min distance between the ray and the segment
    // defined by v0 and v1
    // It can also set two optional targets :
    // - The closest point on the ray
    // - The closest point on the segment

    let seg_center = (v0 + v1) * T::half();
    let seg_dir = (v1 - v0).normalize();
    let diff = self.origin - seg_center;

    let seg_length = v0.distance_to(v1) * T::half();
    let a01 = -self.direction.dot(seg_dir);
    let b0 = diff.dot(self.direction);
    let b1 = -diff.dot(seg_dir);
    let c = diff.length2();
    let det = (T::one() - a01 * a01).abs();
    // let s0, s1, sqrDist, extDet;
    let mut s0 = T::zero();
    let mut s1 = T::zero();
    let sq_dist;

    if det > T::zero() {
      // The ray and segment are not parallel.

      s0 = a01 * b1 - b0;
      s1 = a01 * b0 - b1;
      let ext_det = seg_length * det;

      if s0 >= T::zero() {
        if s1 >= -ext_det {
          if s1 <= ext_det {
            // region 0
            // Minimum at interior points of ray and segment.
            let inv_det = T::one() / det;
            s0 *= inv_det;
            s1 *= inv_det;
            sq_dist =
              s0 * (s0 + a01 * s1 + T::two() * b0) + s1 * (a01 * s0 + s1 + T::two() * b1) + c;
          } else {
            // region 1
            s1 = seg_length;
            s0 = T::zero().max(-(a01 * s1 + b0));
            sq_dist = -s0 * s0 + s1 * (s1 + T::two() * b1) + c;
          }
        } else {
          // region 5
          s1 = -seg_length;
          s0 = T::zero().max(-(a01 * s1 + b0));
          sq_dist = -s0 * s0 + s1 * (s1 + T::two() * b1) + c;
        }
      } else if s1 <= -ext_det {
        // region 4
        s0 = T::zero().max(-(-a01 * seg_length + b0));
        s1 = if s0 > T::zero() {
          -seg_length
        } else {
          (-seg_length).max(-b1).min(seg_length)
        };
        sq_dist = -s0 * s0 + s1 * (s1 + T::two() * b1) + c;
      } else if s1 <= ext_det {
        // region 3
        s0 = T::zero();
        s1 = (-seg_length).max(-b1).min(seg_length);
        sq_dist = s1 * (s1 + T::two() * b1) + c;
      } else {
        // region 2
        s0 = T::zero().max(-(a01 * seg_length + b0));
        s1 = if s0 > T::zero() {
          seg_length
        } else {
          (-seg_length).max(-b1).min(seg_length)
        };
        sq_dist = -s0 * s0 + s1 * (s1 + T::two() * b1) + c;
      }
    } else {
      // Ray3 and segment are parallel.
      let s1 = if a01 > T::zero() {
        -seg_length
      } else {
        seg_length
      };
      let s0 = T::zero().max(-(a01 * s1 + b0));
      sq_dist = -s0 * s0 + s1 * (s1 + T::two() * b1) + c;
    }

    (
      sq_dist,
      self.direction * s0 + self.origin,
      seg_dir * s1 + seg_center,
    )
  }
}
