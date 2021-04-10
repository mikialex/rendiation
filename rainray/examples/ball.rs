use rainray::*;
use rendiation_algebra::{IntoNormalizedVector, Mat4, PerspectiveProjection, Projection};

fn main() {
  let mut renderer = Renderer::new(PathTraceIntegrator::default());
  let perspective = PerspectiveProjection::default();
  let mut camera = Camera::new();
  camera.matrix = Mat4::lookat(
    Vec3::new(0., 7., 10.),
    Vec3::new(0., 5., 0.),
    Vec3::new(0., 1., 0.),
  );
  perspective.update_projection(&mut camera.projection_matrix);

  let mut frame = Frame::new(500, 500);
  let mut scene = Scene::new();
  scene
    .model(Model::new(
      Sphere::new(Vec3::new(0., 5., 0.), 4.0), // main ball
      PhysicalMaterial {
        specular: Specular {
          roughness: 0.01,
          metallic: 0.9,
          ior: 1.6,
          normal_distribution_model: Beckmann,
          geometric_shadow_model: CookTorrance,
          fresnel_model: Schlick,
        },
        diffuse: Diffuse {
          albedo: Vec3::new(0.1, 0.3, 0.3),
          diffuse_model: Lambertian,
        },
      },
    ))
    .model(Model::new(
      Plane::new(Vec3::new(0., 1.0, 0.).into_normalized(), 0.0), // ground
      Diffuse {
        albedo: Vec3::new(0.3, 0.4, 0.8),
        diffuse_model: Lambertian,
      },
    ))
    .model(Model::new(
      Sphere::new(Vec3::new(3., 2., 2.), 2.0),
      Diffuse {
        albedo: Vec3::new(0.4, 0.8, 0.2),
        diffuse_model: Lambertian,
      },
    ))
    .model(Model::new(
      Sphere::new(Vec3::new(-3., 2., 4.), 1.0),
      Diffuse {
        albedo: Vec3::new(1.0, 0.7, 0.0),
        diffuse_model: Lambertian,
      },
    ))
    .light(PointLight {
      position: Vec3::new(8., 8., 6.),
      intensity: Vec3::new(80., 80., 80.),
    })
    .environment(GradientEnvironment {
      // top_intensity: Vec3::splat(0.01),
      // bottom_intensity: Vec3::new(0., 0., 0.),
      top_intensity: Vec3::new(0.4, 0.4, 0.4),
      bottom_intensity: Vec3::new(0.8, 0.8, 0.6),
    });

  renderer.render(&camera, &scene, &mut frame);

  frame.write_result("ball");
}
