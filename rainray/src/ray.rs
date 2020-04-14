use crate::math::*;

pub static MAX_RAY_HIT_DISTANCE: f32 = 1000.0;
pub static EPS: f32 = 0.00001;

pub type RayIntersectAble = dyn IntersectAble<Ray, Option<Intersection>>;

pub struct Intersection {
    pub distance: f32,
    pub hit_position: Vec3,
    pub hit_normal: Vec3,
}
