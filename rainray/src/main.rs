mod bvh;
mod camera;
mod environment;
mod frame;
mod light;
mod material;
mod model;
mod ray;
mod renderer;
mod scene;
mod math;

use crate::camera::*;
use crate::environment::*;
use crate::frame::*;
use crate::light::*;
use crate::material::*;
use crate::math::*;
use crate::renderer::*;
use crate::scene::*;
use std::rc::Rc;

use std::env;

fn main() {
    let renderer = Renderer::new();
    let camera = Camera::new();
    let mut frame = Frame::new(500, 500);
    let scene = Scene {
        models: vec![
            Rc::new(model::Model::new(
                Box::new(Sphere::new((-1., -1., -5.), 3.0)),
                Material::new(),
            )),
            Rc::new(model::Model::new(
                Box::new(Sphere {
                    center: Vec3::new(0., 4., -5.),
                    radius: 1.5,
                }),
                *Material::new().color(0.6, 0.4, 0.8),
            )),
            Rc::new(model::Model {
                geometry: Box::new(Sphere {
                    center: Vec3 {
                        x: 3.,
                        y: -1.,
                        z: -5.,
                    },
                    radius: 2.,
                }),
                material: *Material::new().color(0.8, 0.6, 0.2),
            }),
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
            top_intensity: Vec3::new(1.0, 1.0, 1.0),
            bottom_intensity: Vec3::new(1.0, 1.0, 1.0),
            // bottom_intensity: Vec3::new(0.9, 0.9, 0.9),
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
