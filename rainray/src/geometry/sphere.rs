use crate::math::*;
use crate::ray::*;

#[derive(Debug, Clone, Copy)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f64,
}

impl Sphere {
    pub fn new(center: (f64, f64, f64), radius: f64) -> Self {
        Sphere {
            center: Vec3::new(center.0, center.1, center.2),
            radius,
        }
    }
}

impl Intersecterable for Sphere {
    fn intersect(&self, ray: &Ray) -> Option<Intersection> {
        let voc = self.center - ray.origin; // Vector from the origin to the sphere center
        let voc_len_sqr = voc.norm(); // The length squared of voc
        let vod_len = Vec3::dot(&voc, &ray.direction); // The length of the projected vector voc into the ray direction

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
                let hit_position = ray.point_at_direction(distance);
                let hit_normal = *(hit_position - self.center).normalize();
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
