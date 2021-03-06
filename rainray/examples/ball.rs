use rainray::*;
use rendiation_algebra::*;

fn main() {
  let mut renderer = Renderer::new(PathTraceIntegrator::default());
  let perspective = PerspectiveProjection::default();
  let mut camera = Camera::new();
  camera.matrix = Mat4::lookat(
    Vec3::new(0., 7., 10.),
    Vec3::new(0., 5., 0.),
    Vec3::new(0., 1., 0.),
  );
  perspective.update_projection::<OpenGL>(&mut camera.projection_matrix);

  let mut frame = Frame::new(500, 500);
  let mut scene = Scene::new();
  scene
    .model_node(
      Sphere::new(Vec3::new(0., 5., 0.), 4.0), // main ball
      PhysicalMaterial {
        specular: Specular {
          roughness: 0.3,
          metallic: 0.9,
          ior: 1.6,
          normal_distribution_model: BlinnPhong,
          geometric_shadow_model: CookTorrance,
          fresnel_model: Schlick,
        },
        diffuse: Diffuse {
          albedo: Vec3::new(0.1, 0.3, 0.3),
          diffuse_model: Lambertian,
        },
      },
    )
    .model_node(
      Plane::new(Vec3::new(0., 1.0, 0.).into_normalized(), 0.0), // ground
      Diffuse {
        albedo: Vec3::new(0.3, 0.4, 0.8),
        diffuse_model: Lambertian,
      },
    )
    .model_node(
      Sphere::new(Vec3::new(3., 2., 2.), 2.0),
      Diffuse {
        albedo: Vec3::new(0.4, 0.8, 0.2),
        diffuse_model: Lambertian,
      },
    )
    .model_node(
      Sphere::new(Vec3::new(-3., 2., 4.), 1.0),
      Diffuse {
        albedo: Vec3::new(1.0, 0.7, 0.0),
        diffuse_model: Lambertian,
      },
    )
    // .create_node(|node, scene| {
    //   node.set_position((8., 8., 6.)).with_light(
    //     scene.create_light(
    //       sceno::PointLight {
    //         intensity: Vec3::new(80., 80., 80.),
    //       }
    //       .to_boxed(),
    //     ),
    //   );
    // })
    .background(GradientBackground {
      // top_intensity: Vec3::splat(0.01),
      // bottom_intensity: Vec3::new(0., 0., 0.),
      top_intensity: Vec3::new(0.4, 0.4, 0.4),
      bottom_intensity: Vec3::new(0.8, 0.8, 0.6),
    });

  renderer.render(&camera, &mut scene, &mut frame);

  frame.write_result("ball");
}
