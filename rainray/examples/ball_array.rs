use rainray::*;

fn main() {
  let mut renderer = Renderer::new(PathTraceIntegrator::default());
  let perspective = PerspectiveProjection::default();
  let mut camera = Camera::new();

  let mut frame = Frame::new(2000, 2000);
  let mut scene = Scene::default();

  scene
    .model(Model::new(
      Sphere::new(Vec3::new(0., -10000., 0.), 10000.0), // ground
      Diffuse {
        albedo: Vec3::new(0.5, 0.4, 0.8),
        diffuse_model: Lambertian,
      },
    ))
    .light(PointLight {
      // front light
      position: Vec3::new(8., 8., 5.),
      intensity: Vec3::splat(40.),
    })
    // .light(PointLight { // back light
    //   position: Vec3::new(-8., 8., -5.),
    //   intensity: Vec3::splat(40.),
    // })
    .environment(GradientEnvironment {
      // top_intensity: Vec3::splat(0.01),
      // bottom_intensity: Vec3::new(0., 0., 0.),
      top_intensity: Vec3::new(0.4, 0.4, 0.4),
      bottom_intensity: Vec3::new(0.8, 0.8, 0.6),
    });

  fn ball(position: Vec3, size: f32, roughness: f32, metallic: f32) -> Model {
    let roughness = if roughness == 0.0 { 0.04 } else { roughness };
    Model::new(
      Sphere::new(position, size),
      PhysicalMaterial {
        specular: Specular {
          roughness,
          metallic,
          ior: 1.5,
          normal_distribution_model: Beckmann,
          geometric_shadow_model: CookTorrance,
          fresnel_model: Schlick,
        },
        diffuse: Diffuse {
          // albedo: Vec3::splat(1.0),
          albedo: Vec3::new(1.0, 0.7, 0.2),
          diffuse_model: Lambertian,
        },
      },
    )
  }

  let r = 0.5;
  let spacing = 0.55;
  let count = 10;

  let width_all = spacing as f32 * 2. * count as f32;

  let start = width_all / -2.0 + spacing;
  let step = spacing * 2.;
  for i in 0..count {
    for j in 0..count {
      scene.model(ball(
        Vec3::new(start + i as f32 * step, j as f32 * step + spacing, 2.0),
        r,
        i as f32 / (count - 1) as f32,
        j as f32 / (count - 1) as f32,
      ));
    }
  }
  *camera.matrix_mut() = Mat4::lookat(
    Vec3::new(0., width_all / 2., 10.),
    Vec3::new(0., width_all / 2., 0.),
    Vec3::new(0., 1., 0.),
  );
  camera.update_by(&perspective);
  // renderer.sample_per_pixel = 200;

  let mut current_path = std::env::current_dir().unwrap();
  println!("working dir {}", current_path.display());
  renderer.render(&camera, &scene, &mut frame);
  current_path.push("result.png");
  let file_target_path = current_path.into_os_string().into_string().unwrap();

  println!("writing file to path: {}", file_target_path);
  frame.write_to_file(&file_target_path);
}
