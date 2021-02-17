use rainray::*;
use rendiation_algebra::IntoNormalizedVector;

fn main() {
  let mut renderer = Renderer::new(PathTraceIntegrator::default());
  // renderer.sample_per_pixel = 1;
  let perspective = PerspectiveProjection::default();
  let mut camera = Camera::new();
  *camera.matrix_mut() = Mat4::lookat(
    Vec3::new(0., 7., 10.),
    Vec3::new(0., 5., 0.),
    Vec3::new(0., 1., 0.),
  );
  camera.update_by(&perspective);

  let mut frame = Frame::new(1200, 1200);
  let mut scene = Scene::default();
  scene
    .model(Model::new(
      Mesh::from_path_obj("/Users/mikialex/testdata/obj/bunny.obj"),
      // Mesh::from_path_obj("C:/Users/mk/Desktop/bunny.obj"),
      // Diffuse {
      //   albedo: Vec3::new(0.3, 0.4, 0.8),
      //   diffuse_model: Lambertian,
      // },
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
          albedo: Vec3::new(0.5, 0.5, 0.5),
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

  frame.write_result("bunny");
}
