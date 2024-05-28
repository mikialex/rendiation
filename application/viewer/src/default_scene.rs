use rendiation_algebra::*;
use rendiation_mesh_core::CommonVertex;
use rendiation_mesh_core::*;
use rendiation_mesh_generator::{
  build_attributes_mesh, CubeMeshParameter, SphereMeshParameter, TessellationConfig,
};
use rendiation_texture_core::{
  create_padding_buffer, GPUBufferImage, Texture2D, TextureFormat, TextureSampler,
};

use crate::*;

pub fn textured_ball(scene: &mut Scene3dWriter) {
  let path = if cfg!(windows) {
    "C:/Users/mk/Desktop/rrf-resource/planets/earth_atmos_2048.jpg"
  } else {
    "/Users/mikialex/Desktop/test.png"
  };
}

// pub fn load_img_cube() -> SceneTextureCube {
//   let path = if cfg!(windows) {
//     [
//       "C:/Users/mk/Desktop/rrf-resource/Park2/posx.jpg",
//       "C:/Users/mk/Desktop/rrf-resource/Park2/negx.jpg",
//       "C:/Users/mk/Desktop/rrf-resource/Park2/posy.jpg",
//       "C:/Users/mk/Desktop/rrf-resource/Park2/negy.jpg",
//       "C:/Users/mk/Desktop/rrf-resource/Park2/posz.jpg",
//       "C:/Users/mk/Desktop/rrf-resource/Park2/negz.jpg",
//     ]
//   } else {
//     [
//       "/Users/mikialex/dev/references/three.js/examples/textures/cube/pisa/px.png",
//       "/Users/mikialex/dev/references/three.js/examples/textures/cube/pisa/nx.png",
//       "/Users/mikialex/dev/references/three.js/examples/textures/cube/pisa/py.png",
//       "/Users/mikialex/dev/references/three.js/examples/textures/cube/pisa/ny.png",
//       "/Users/mikialex/dev/references/three.js/examples/textures/cube/pisa/pz.png",
//       "/Users/mikialex/dev/references/three.js/examples/textures/cube/pisa/nz.png",
//     ]
//   };

//   SceneTextureCubeImpl {
//     faces: path.map(load_tex),
//   }
//   .into()
// }

// pub fn load_default_scene(scene: &Scene) {
//   let path = if cfg!(windows) {
//     "C:/Users/mk/Desktop/rrf-resource/planets/earth_atmos_2048.jpg"
//   } else {
//     "/Users/mikialex/Desktop/test.png"
//   };

//   let texture = TextureWithSamplingData {
//     texture: load_tex(path).into_ptr(),
//     sampler: TextureSampler::tri_linear_repeat().into_ptr(),
//   };

//   scene.set_background(Some(SceneBackGround::Solid(SolidBackground {
//     intensity: Vec3::new(0.1, 0.1, 0.1),
//   })));
//   // scene.set_background(Some(SceneBackGround::Env(EnvMapBackground {
//   //   texture: load_img_cube(),
//   // })));

//   {
//     let mesh = build_scene_mesh(|builder| {
//       builder.triangulate_parametric(
//         &SphereMeshParameter::default().make_surface(),
//         TessellationConfig { u: 16, v: 16 },
//         true,
//       );
//     });

//     let material = PhysicalSpecularGlossinessMaterial {
//       albedo: Vec3::splat(1.),
//       albedo_texture: texture.clone().into(),
//       ..Default::default()
//     };
//     let material = MaterialEnum::PhysicalSpecularGlossiness(material.into());

//     let child = scene.create_root_child();
//     child.set_local_matrix(Mat4::translate((2., 0., 3.)));

//     let model = StandardModel::new(material, mesh);
//     let model = ModelEnum::Standard(model.into());
//     let model = SceneModelImpl::new(model, child);
//     let _ = scene.insert_model(model.into());
//   }

//   {
//     let cube = CubeMeshParameter {
//       width: 1.,
//       height: 2.,
//       depth: 3.,
//     };
//     let mesh = build_scene_mesh(|builder| {
//       for face in cube.make_faces() {
//         builder.triangulate_parametric(&face, TessellationConfig { u: 2, v: 3 }, true);
//       }
//     });

//     let material = PhysicalSpecularGlossinessMaterial {
//       albedo: Vec3::splat(1.),
//       albedo_texture: texture.clone().into(),
//       ..Default::default()
//     };
//     let material = MaterialEnum::PhysicalSpecularGlossiness(material.into());
//     let child = scene.create_root_child();

//     let model = StandardModel::new(material, mesh);
//     let model = ModelEnum::Standard(model.into());
//     let model = SceneModelImpl::new(model, child);
//     let _ = scene.insert_model(model.into());
//   }

//   {
//     let mesh = build_scene_mesh(|builder| {
//       builder.triangulate_parametric(
//         &SphereMeshParameter::default().make_surface(),
//         TessellationConfig { u: 16, v: 16 },
//         true,
//       );
//     });

//     let mesh = TransformInstancedSceneMesh {
//       mesh,
//       transforms: vec![
//         Mat4::translate((10., 0., 0.)),
//         Mat4::translate((10., 0., 2.)),
//         Mat4::translate((10., 0., 4.)),
//         Mat4::translate((10., 0., 6.)),
//       ],
//     }
//     .into_ptr();
//     let mesh = MeshEnum::TransformInstanced(mesh);

//     let material = PhysicalSpecularGlossinessMaterial {
//       albedo: Vec3::splat(1.),
//       albedo_texture: texture.into(),
//       ..Default::default()
//     };
//     let material = MaterialEnum::PhysicalSpecularGlossiness(material.into());
//     let child = scene.create_root_child();

//     let model = StandardModel::new(material, mesh);
//     let model = ModelEnum::Standard(model.into());
//     let model = SceneModelImpl::new(model, child);
//     let _ = scene.insert_model(model.into());
//   }

//   let up = Vec3::new(0., 1., 0.);

//   {
//     let camera = PerspectiveProjection::default();
//     let camera = CameraProjectionEnum::Perspective(camera);
//     let camera_node = scene.create_root_child();
//     camera_node.set_local_matrix(Mat4::lookat(Vec3::splat(3.), Vec3::splat(0.), up));
//     let camera = SceneCameraImpl::new(camera, camera_node).into_ptr();
//     let _ = scene.insert_camera(camera.clone());
//     scene.set_active_camera(camera.into());
//   }

//   {
//     let camera = PerspectiveProjection::default();
//     let camera = CameraProjectionEnum::Perspective(camera);
//     let camera_node = scene.create_root_child();
//     camera_node.set_local_matrix(Mat4::lookat(Vec3::splat(3.), Vec3::splat(0.), up));
//     let camera = SceneCameraImpl::new(camera, camera_node).into_ptr();
//     let _ = scene.insert_camera(camera);
//   }

//   let directional_light_node = scene.create_root_child();
//   directional_light_node.set_local_matrix(Mat4::lookat(Vec3::splat(300.), Vec3::splat(0.), up));
//   let directional_light = DirectionalLight {
//     illuminance: 5.,
//     color_factor: Vec3::one(),
//   };
//   let directional_light = LightEnum::DirectionalLight(directional_light.into());
//   let directional_light = SceneLightImpl::new(directional_light, directional_light_node);
//   scene.insert_light(directional_light.into());

//   let directional_light_node = scene.create_root_child();
//   directional_light_node.set_local_matrix(Mat4::lookat(
//     Vec3::new(30., 300., -30.),
//     Vec3::splat(0.),
//     up,
//   ));
//   let directional_light = DirectionalLight {
//     illuminance: 5.,
//     color_factor: Vec3::new(5., 3., 2.) / Vec3::splat(5.),
//   };
//   let directional_light = LightEnum::DirectionalLight(directional_light.into());
//   let directional_light = SceneLightImpl::new(directional_light, directional_light_node);
//   scene.insert_light(directional_light.into());

//   let point_light_node = scene.create_root_child();
//   point_light_node.set_local_matrix(Mat4::translate((2., 2., 2.)));
//   let point_light = PointLight {
//     color_factor: Vec3::new(5., 3., 2.) / Vec3::splat(5.),
//     luminance_intensity: 5.,
//     cutoff_distance: 40.,
//   };
//   let point_light = LightEnum::PointLight(point_light.into());
//   let point_light = SceneLightImpl::new(point_light, point_light_node);
//   scene.insert_light(point_light.into());

//   let spot_light_node = scene.create_root_child();
//   spot_light_node.set_local_matrix(Mat4::lookat(Vec3::new(-5., 5., 5.), Vec3::splat(0.), up));
//   let spot_light = SpotLight {
//     luminance_intensity: 180.,
//     color_factor: Vec3::new(1., 0., 0.),
//     cutoff_distance: 40.,
//     half_cone_angle: Deg::by(5. / 2.).to_rad(),
//     half_penumbra_angle: Deg::by(5. / 2.).to_rad(),
//   };
//   let spot_light = LightEnum::SpotLight(spot_light.into());
//   let spot_light = SceneLightImpl::new(spot_light, spot_light_node);
//   scene.insert_light(spot_light.into());

//   // stress_test2(scene);
// }

pub fn load_stress_test(scene: &mut Scene3dWriter) {
  let material = PhysicalSpecularGlossinessMaterialDataView {
    albedo: Vec3::splat(1.),
    albedo_texture: None,
    ..Default::default()
  }
  .write(&mut scene.pbr_sg_mat_writer);
  let material = SceneMaterialDataView::PbrSGMaterial(material);
  for i in 0..10 {
    let i_parent = scene.create_root_child();
    scene.set_local_matrix(i_parent, Mat4::translate((i as f32, 0., 0.)));
    for j in 0..10 {
      let j_parent = scene.create_child(i_parent);
      scene.set_local_matrix(j_parent, Mat4::translate((0., 0., j as f32)));
      for k in 0..1 {
        let node = scene.create_child(j_parent);
        scene.set_local_matrix(node, Mat4::translate((0., k as f32, 0.)));

        let cube = CubeMeshParameter {
          width: 0.2,
          height: 0.2,
          depth: 0.2,
        };
        let mesh = build_attributes_mesh(|builder| {
          for face in cube.make_faces() {
            builder.triangulate_parametric(&face, TessellationConfig { u: 2, v: 3 }, true);
          }
        });
        let mesh = scene.write_attribute_mesh(mesh.build());

        scene.create_scene_model(material, mesh, node);
      }
    }
  }
}
