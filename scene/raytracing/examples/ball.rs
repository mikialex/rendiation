use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_lighting_transport::*;
use rendiation_scene_raytracing::*;
mod utils;
use utils::*;

fn main() {
  setup_active_plane(Default::default());
  let mut renderer = PathTraceIntegrator::default();

  let mut frame = make_frame(500, 500);

  let mut scene = SceneImpl::new().0;

  let perspective = make_perspective();
  let perspective = CameraProjectionEnum::Perspective(perspective);
  let camera = SceneCameraImpl::new(perspective, scene.create_root_child()).into_ptr();
  camera.read().node.set_local_matrix(Mat4::lookat(
    Vec3::new(0., 8., 10.),
    Vec3::new(0., 5., 0.),
    Vec3::new(0., 1., 0.),
  ));

  scene
    .model_node(
      Sphere::new(Vec3::new(0., 5., 0.), 4.0), // main ball
      RtxPhysicalMaterial {
        specular: Specular {
          roughness: 0.3,
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

  let mut source = scene.build_traceable();
  let camera = source.build_camera(&camera);
  renderer.render(&camera, &mut source, &mut frame);

  write_frame(&frame, "ball");
}
