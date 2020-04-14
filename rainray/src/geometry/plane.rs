use crate::ray::*;

use crate::math::*;

struct Plane {
    center: Vec3,
    normal: Vec3,
}

impl Plane {
    pub fn new(center: Vec3, normal: Vec3) -> Plane {
        Plane { center, normal }
    }
}

impl Intersecterable for Plane {
    fn intersect(&self, _ray: &Ray) -> Option<Intersection> {
        unimplemented!();
    }
}
