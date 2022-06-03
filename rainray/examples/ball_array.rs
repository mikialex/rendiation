use rainray::*;
use rendiation_algebra::*;

fn main() {
  let mut renderer = Renderer::new(PathTraceIntegrator::default());

  let mut frame = Frame::new(1000, 1000);
  let mut scene = Scene::new();

  let perspective = PerspectiveProjection::default();
  let camera = SceneCamera::new(perspective, scene.root().create_child());

  scene
    .model_node(
      Plane::new(Vec3::new(0., 1.0, 0.).into_normalized(), 0.0), // ground
      Diffuse {
        albedo: Vec3::new(0.5, 0.4, 0.8),
        diffuse_model: Lambertian,
      },
    )
    // .create_node(|node, scene| {
    //   node.set_position((8., 8., 5.)).with_light(
    //     scene.create_light(
    //       sceno::PointLight {
    //         intensity: Vec3::splat(40.),
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

  fn ball(
    scene: &mut Scene<RayTracingScene>,
    position: Vec3<f32>,
    size: f32,
    roughness: f32,
    metallic: f32,
  ) {
    let roughness = if roughness == 0.0 { 0.04 } else { roughness };
    scene.model_node(
      Sphere::new(position, size),
      RtxPhysicalMaterial {
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
    );
  }

  let r = 0.5;
  let spacing = 0.55;
  let count = 10;

  let width_all = spacing as f32 * 2. * count as f32;

  let start = width_all / -2.0 + spacing;
  let step = spacing * 2.;
  for i in 0..count {
    for j in 0..count {
      ball(
        &mut scene,
        Vec3::new(start + i as f32 * step, j as f32 * step + spacing, 2.0),
        r,
        i as f32 / (count - 1) as f32,
        j as f32 / (count - 1) as f32,
      );
    }
  }

  camera.node.mutate(|node| {
    node.local_matrix = Mat4::lookat(
      Vec3::new(0., width_all / 2., 10.),
      Vec3::new(0., width_all / 2., 0.),
      Vec3::new(0., 1., 0.),
    );
  });

  renderer.render(&camera, &mut scene, &mut frame);
  frame.write_result("ball_array");
}
