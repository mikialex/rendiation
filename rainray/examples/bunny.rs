use rainray::*;
use rendiation_algebra::*;

fn main() {
  let mut renderer = Renderer::new(PathTraceIntegrator::default());
  // renderer.sample_per_pixel = 1;
  let mut perspective = PerspectiveProjection::default();
  let mut camera = Camera::new();
  camera.matrix = Mat4::lookat(
    Vec3::new(0., 8., 10.),
    Vec3::new(0., 5., 0.),
    Vec3::new(0., 1., 0.),
  );
  perspective.fov = 65.;
  perspective.update_projection::<OpenGL>(&mut camera.projection_matrix);

  let mut frame = Frame::new(600, 600);
  let mut scene = Scene::new();
  scene
    .model_node_with_modify(
      TriangleMesh::from_path_obj("/Users/mikialex/testdata/obj/bunny.obj"),
      // TriangleMesh::from_path_obj("C:/Users/mk/Desktop/bunny.obj"),
      // Diffuse {
      //   albedo: Vec3::new(0.3, 0.4, 0.8),
      //   diffuse_model: Lambertian,
      // },
      PhysicalMaterial {
        specular: Specular {
          roughness: 0.001,
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
      |node| {
        node.local_matrix = Mat4::translate(1., 0., 0.)
        // node.local_matrix = Mat4::translate(0., 2., 0.) * Mat4::rotate_y(3.)
      },
    )
    .model_node_with_modify(
      Plane::new(Vec3::new(0., 1.0, 0.).into_normalized(), 0.0), // ground
      Diffuse {
        albedo: Vec3::new(0.3, 0.4, 0.8),
        diffuse_model: Lambertian,
      },
      |node| {
        node.local_matrix = Mat4::translate(0., 1.0, 0.)
        // node.local_matrix = Mat4::translate(0., 2., 0.) * Mat4::rotate_y(3.)
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

  frame.write_result("bunny");
}
