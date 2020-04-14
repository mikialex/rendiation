use crate::math::*;

extern crate rand as randx;

pub fn rand() -> f64 {
    randx::random()
}

pub fn rand2() -> (f64, f64) {
    (randx::random(), randx::random())
}

pub fn cosine_sample_hemisphere(normal: &Vec3) -> Vec3 {
    // let r1 = rand() * std::f64::consts::PI;
    // let r2 = rand();
    // let r3 = r2.sqrt();

    // let mut u = if normal.x.abs() > 0.1 {
    //     Vec3::new(normal.z, 0.0, -normal.x)
    // } else {
    //     Vec3::new(0.0, -normal.z, normal.y)
    // };
    // u.normalize();
    // // vec3 u = normalize((abs(normal.x) > 0.1) ? vec3(normal.z, 0.0, -normal.x) : vec3(0.0, -normal.z, normal.y));
    // (u * r1.cos() + Vec3::cross(&normal, &u) * r1.sin()) * r3 + *normal * (1.0 - r2).sqrt()

    // let u1 = rand();
    // let u2 = rand();
    // let r = (1. - u1 * u1).sqrt();
    // let phi = 2. * std::f64::consts::PI * u2;
    // *Vec3::new(phi.cos() * r, phi.sin() * r, u2).normalize()

    // *Vec3::new(rand(), rand(), rand()).normalize()

    Vec3::new(0.0, 1.0, 0.0)
}

pub fn rand_point_in_unit_sphere() -> Vec3 {
    loop {
        let test_point = Vec3::new(rand() * 2.0 - 1.0, rand() * 2.0 - 1.0, rand() * 2.0 - 1.0);
        if test_point.length() <= 1. {
            break test_point;
        }
    }
}
