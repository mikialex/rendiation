use crate::math::*;

#[derive(Debug, Clone, Copy)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

impl Ray {
    pub fn point_at_direction(&self, distance: f64) -> Vec3 {
        self.origin + self.direction * distance
    }

    #[allow(dead_code)]
    pub fn new(origin: Vec3, direction: Vec3) -> Ray {
        Ray { origin, direction }
    }

    pub fn from_point_to_point(origin: &Vec3, target: &Vec3) -> Ray {
        Ray {
            origin: origin.clone(),
            direction: *(*target - *origin).normalize(),
        }
    }

    pub fn copy_from(&mut self, ray: &Ray) {
        self.origin.copy_from(&ray.origin);
        self.direction.copy_from(&ray.direction);
    }
}

pub static MAX_RAY_HIT_DISTANCE: f64 = 1000.0;
pub static EPS: f64 = 0.00001;

pub trait Intersecterable {
    fn intersect(&self, ray: &Ray) -> Option<Intersection>;
}

pub struct Intersection {
    pub distance: f64,
    pub hit_position: Vec3,
    pub hit_normal: Vec3,
}
