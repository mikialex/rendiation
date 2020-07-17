#![allow(unused)]
mod environment;
mod frame;
mod integrator;
mod light;
mod material;
mod math;
mod model;
mod ray;
mod renderer;
mod scene;

use crate::environment::*;
use crate::frame::*;
use crate::light::*;
use crate::material::*;
use crate::math::*;
use crate::renderer::*;
use crate::scene::*;
use integrator::*;
use rendiation_math::Mat4;
use rendiation_render_entity::*;
use std::env;
use std::sync::Arc;

fn main() {
  let mut renderer = Renderer::new(PathTraceIntegrator::new());
  let mut camera = PerspectiveCamera::new();
  camera.world_matrix = Mat4::lookat(
    Vec3::new(0., 0., 10.),
    Vec3::new(0., 0., 0.),
    Vec3::new(0., 1., 0.),
  ) * Mat4::translate(0., 5., 0.);
  camera.update_projection();

  let mut frame = Frame::new(500, 500);
  let scene = Scene {
    models: vec![
      Arc::new(model::Model::new(
        Sphere::new((0., 5., 0.).into(), 4.0), // main ball
        Lambertian::new(),
      )),
      Arc::new(model::Model::new(
        Sphere::new((0., -10000., 0.).into(), 10000.0), // ground
        *Lambertian::new().albedo(0.3, 0.4, 0.8),
      )),
      Arc::new(model::Model::new(
        Sphere::new((3., 2., 2.).into(), 2.0),
        *Lambertian::new().albedo(0.4, 0.8, 0.2),
      )),
      Arc::new(model::Model::new(
        Sphere::new((-3., 2., 4.).into(), 1.0),
        *Lambertian::new().albedo(1.0, 0.1, 0.0),
      )),
    ],
    point_lights: vec![PointLight {
      position: Vec3 {
        x: -200.,
        y: -200.,
        z: 100.,
      },
      color: Vec3::new(1.0, 1.0, 1.0),
    }],
    env: Box::new(GradientEnvironment {
      top_intensity: Vec3::new(0.4, 0.4, 0.4),
      bottom_intensity: Vec3::new(0.8, 0.8, 0.6),
    }),
  };

  let mut current_path = env::current_dir().unwrap();
  println!("working dir {}", current_path.display());
  renderer.render(&camera, &scene, &mut frame);
  current_path.push("result.png");
  let file_target_path = current_path.into_os_string().into_string().unwrap();

  println!("writing file to path: {}", file_target_path);
  frame.write_to_file(&file_target_path);
}
