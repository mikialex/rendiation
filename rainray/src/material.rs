use crate::frame::*;
use crate::math::*;
use crate::ray::*;

#[derive(Debug, Clone, Copy)]
pub struct Material {
    pub diffuse_color: Color,
    pub emissive: Vec3,
}

impl Material {
    pub fn new() -> Material {
        Material {
            diffuse_color: Color::new(0.95, 0.95, 0.95),
            emissive: Vec3::new(0.0, 0.0, 0.0),
        }
    }

    pub fn color(&mut self, r: f64, g: f64, b: f64) -> &Self {
        self.diffuse_color.r = r;
        self.diffuse_color.g = g;
        self.diffuse_color.b = b;
        self
    }

    pub fn next_ray(&self, into_ray: &Ray, intersection: &Intersection) -> Ray {
        // Ray::new(
        //     intersection.hit_position,
        //     cosine_sample_hemisphere(&intersection.hit_normal),
        // )
        // Ray::new(
        //     intersection.hit_position,
        //     Vec3::reflect(&intersection.hit_normal, &into_ray.direction),
        // )

        Ray::from_point_to_point(
            &intersection.hit_position,
            &(intersection.hit_position + intersection.hit_normal + rand_point_in_unit_sphere()),
        )

        // Ray::from_point_to_point(
        //     &intersection.hit_position,
        //     &(intersection.hit_position
        //         + intersection.hit_normal
        //         + 0.5 * rand_point_in_unit_sphere()
        //         + Vec3::reflect(&intersection.hit_normal, &into_ray.direction)),
        // )
    }

    pub fn collect_energy(&self, look_up_ray: &Ray) -> Vec3 {
        self.emissive
    }

    pub fn brdf_importance_pdf(
        &self,
        intersection: &Intersection,
        in_ray: &Ray,
        out_ray: &Ray,
    ) -> f64 {
        1.
    }

    pub fn BRDF(&self, intersection: &Intersection, in_ray: &Ray, out_ray: &Ray) -> f64 {
        let w_m = (-in_ray.direction + out_ray.direction).normalize();
        0.8
    }
}
