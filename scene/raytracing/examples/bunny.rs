use std::sync::Arc;

use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_lighting_transport::*;
use rendiation_scene_raytracing::*;

mod utils;
use utils::*;

fn main() {
  setup_active_plane(Default::default());
  let mut renderer = PathTraceIntegrator::default();
  // renderer.sample_per_pixel = 1;

  let mut frame = make_frame(600, 600);

  let mut scene = SceneImpl::new().0;

  let perspective = PerspectiveProjection {
    fov: Deg::by(65.),
    ..Default::default()
  };
  let perspective = CameraProjectionEnum::Perspective(perspective);
  let camera = SceneCameraImpl::new(perspective, scene.create_root_child()).into_ptr();
  camera.read().node.set_local_matrix(Mat4::lookat(
    Vec3::new(0., 8., 10.),
    Vec3::new(0., 5., 0.),
    Vec3::new(0., 1., 0.),
  ));

  scene
    .model_node_with_modify(
      Arc::new(TriangleMesh::from_path_obj(
        "/Users/mikialex/testdata/obj/bunny.obj",
      )),
      // TriangleMesh::from_path_obj("C:/Users/mk/Desktop/bunny.obj"),
      // Diffuse {
      //   albedo: Vec3::new(0.3, 0.4, 0.8),
      //   diffuse_model: Lambertian,
      // },
      RtxPhysicalMaterial {
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
        node.set_local_matrix(Mat4::translate((1., 0., 0.)))
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
        node.set_local_matrix(Mat4::translate((0., 1.0, 0.)))
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

  let mut source = scene.build_traceable();
  let camera = source.build_camera(&camera);
  renderer.render(&camera, &mut source, &mut frame);

  write_frame(&frame, "bunny");
}
