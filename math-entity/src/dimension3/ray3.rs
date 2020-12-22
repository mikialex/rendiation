use crate::{HyperRay, LineSegment, Positioned};
use rendiation_math::*;

pub type Ray3 = HyperRay<f32, 3>;

impl SpaceEntity<f32, 3> for Ray3 {
  #[inline]
  fn apply_matrix(&mut self, mat: SquareMatrixType<f32, 3>) -> &mut Self {
    let origin = self.origin * mat;
    let direction = self.direction.transform_direction(mat);
    *self = Self::new(origin, direction);
    self
  }
}

impl Ray3 {
  pub fn from_point_to_point(origin: Vec3<f32>, target: Vec3<f32>) -> Self {
    Ray3::new(origin, (target - origin).into_normalized())
  }

  pub fn at(&self, distance: f32) -> Vec3<f32> {
    self.origin + self.direction * distance
  }

  pub fn distance_sq_to_point(&self, point: Vec3<f32>) -> f32 {
    let oc = point - self.origin;
    let tca = oc.dot(self.direction);
    oc.dot(oc) - tca * tca
  }

  pub fn distance_sq_to_segment<T: Positioned<f32, 3>>(
    &self,
    line: LineSegment<T>,
  ) -> (f32, Vec3<f32>, Vec3<f32>) {
    // (distance_sq_to_segment, optionalPointOnRay, optionalPointOnSegment)
    let v0 = line.start.position();
    let v1 = line.end.position();

    // from http://www.geometrictools.com/GTEngine/Include/Mathematics/GteDistRaySegment.h
    // It returns the min distance between the ray and the segment
    // defined by v0 and v1
    // It can also set two optional targets :
    // - The closest point on the ray
    // - The closest point on the segment

    let seg_center = (v0 + v1) * 0.5;
    let seg_dir = (v1 - v0).normalize();
    let diff = self.origin - seg_center;

    let seg_length = v0.distance(v1) * 0.5;
    let a01 = -self.direction.dot(seg_dir);
    let b0 = diff.dot(self.direction);
    let b1 = -diff.dot(seg_dir);
    let c = diff.length2();
    let det = (1.0 - a01 * a01).abs();
    // let s0, s1, sqrDist, extDet;
    let mut s0 = 0.;
    let mut s1 = 0.;
    #[allow(unused_assignments)]
    let mut sq_dist = 0.;

    if det > 0. {
      // The ray and segment are not parallel.

      s0 = a01 * b1 - b0;
      s1 = a01 * b0 - b1;
      let ext_det = seg_length * det;

      if s0 >= 0. {
        if s1 >= -ext_det {
          if s1 <= ext_det {
            // region 0
            // Minimum at interior points of ray and segment.
            let inv_det = 1. / det;
            s0 *= inv_det;
            s1 *= inv_det;
            sq_dist = s0 * (s0 + a01 * s1 + 2. * b0) + s1 * (a01 * s0 + s1 + 2. * b1) + c;
          } else {
            // region 1
            s1 = seg_length;
            s0 = 0.0_f32.max(-(a01 * s1 + b0));
            sq_dist = -s0 * s0 + s1 * (s1 + 2. * b1) + c;
          }
        } else {
          // region 5
          s1 = -seg_length;
          s0 = 0.0_f32.max(-(a01 * s1 + b0));
          sq_dist = -s0 * s0 + s1 * (s1 + 2. * b1) + c;
        }
      } else if s1 <= -ext_det {
        // region 4
        s0 = 0.0_f32.min(-(-a01 * seg_length + b0));
        s1 = if s0 > 0. {
          -seg_length
        } else {
          (-seg_length).max(-b1).min(seg_length)
        };
        sq_dist = -s0 * s0 + s1 * (s1 + 2. * b1) + c;
      } else if s1 <= ext_det {
        // region 3
        s0 = 0.;
        s1 = (-seg_length).max(-b1).min(seg_length);
        sq_dist = s1 * (s1 + 2. * b1) + c;
      } else {
        // region 2
        s0 = 0.0_f32.max(-(a01 * seg_length + b0));
        s1 = if s0 > 0. {
          seg_length
        } else {
          (-seg_length).max(-b1).min(seg_length)
        };
        sq_dist = -s0 * s0 + s1 * (s1 + 2. * b1) + c;
      }
    } else {
      // Ray3 and segment are parallel.
      let s1 = if a01 > 0. { -seg_length } else { seg_length };
      let s0 = 0.0_f32.max(-(a01 * s1 + b0));
      sq_dist = -s0 * s0 + s1 * (s1 + 2. * b1) + c;
    }

    (
      sq_dist,
      self.direction * s0 + self.origin,
      seg_dir * s1 + seg_center,
    )
  }
}
