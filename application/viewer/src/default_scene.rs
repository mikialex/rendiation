use rendiation_algebra::*;
use rendiation_area_lighting::*;
use rendiation_mesh_generator::{
  build_attributes_mesh, CubeMeshParameter, IntoTransformed3D, ParametricPlane,
  SphereMeshParameter, TessellationConfig,
};

use crate::*;

pub fn load_default_scene(writer: &mut SceneWriter, _viewer_scene: &Viewer3dSceneCtx) {
  // test_ltc_lighting(writer);

  // textured ball
  {
    let attribute_mesh = build_attributes_mesh(|builder| {
      builder.triangulate_parametric(
        &SphereMeshParameter::default().make_surface(),
        TessellationConfig { u: 16, v: 16 },
        true,
      );
    })
    .build();
    let attribute_mesh = writer.write_attribute_mesh(attribute_mesh).mesh;

    let texture = textured_example_tex(writer);
    let material = PhysicalMetallicRoughnessMaterialDataView {
      base_color: Vec3::splat(0.8),
      base_color_texture: Some(texture),
      roughness: 0.1,
      metallic: 0.8,
      ..Default::default()
    }
    .write(&mut writer.pbr_mr_mat_writer);
    let material = SceneMaterialDataView::PbrMRMaterial(material);
    let child = writer.create_root_child();
    writer.create_scene_model(material, attribute_mesh, child);
  }

  // cube
  {
    let cube = CubeMeshParameter {
      width: 1.,
      height: 2.,
      depth: 3.,
    };
    let attribute_mesh = build_attributes_mesh(|builder| {
      for face in cube.make_faces() {
        builder.triangulate_parametric(&face, TessellationConfig { u: 2, v: 3 }, true);
      }
    })
    .build();
    let attribute_mesh = writer.write_attribute_mesh(attribute_mesh).mesh;

    let material = PhysicalSpecularGlossinessMaterialDataView {
      albedo: Vec3::splat(1.),
      ..Default::default()
    }
    .write(&mut writer.pbr_sg_mat_writer);
    let material = SceneMaterialDataView::PbrSGMaterial(material);

    let child = writer.create_root_child();
    writer.set_local_matrix(
      child,
      Mat4::translate((2., 0., 3.)) * Mat4::scale((1., 2., 1.)),
    );

    writer.create_scene_model(material, attribute_mesh, child);
  }

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

  let up = Vec3::new(0., 1., 0.);

  // add another camera for camera related helper testing
  {
    let camera_node = writer.create_root_child();
    writer.set_local_matrix(
      camera_node,
      Mat4::lookat(Vec3::splat(3.), Vec3::splat(0.), up),
    );
    writer
      .camera_writer
      .component_value_writer::<SceneCameraPerspective>(Some(PerspectiveProjection::default()))
      .component_value_writer::<SceneCameraBelongsToScene>(Some(writer.scene.into_raw()))
      .component_value_writer::<SceneCameraNode>(Some(camera_node.into_raw()))
      .new_entity();
  }

  {
    let directional_light_node = writer.create_root_child();
    writer.set_local_matrix(
      directional_light_node,
      Mat4::lookat(Vec3::splat(300.), Vec3::splat(0.), up),
    );
    DirectionalLightDataView {
      illuminance: Vec3::splat(5.),
      node: directional_light_node,
      scene: writer.scene,
    }
    .write(&mut writer.directional_light_writer);
  }

  {
    let directional_light_node = writer.create_root_child();
    writer.set_local_matrix(
      directional_light_node,
      Mat4::lookat(Vec3::new(30., 300., -30.), Vec3::splat(0.), up),
    );
    DirectionalLightDataView {
      illuminance: Vec3::new(5., 3., 2.) * 5.,
      node: directional_light_node,
      scene: writer.scene,
    }
    .write(&mut writer.directional_light_writer);
  }

  {
    let point_light_node = writer.create_root_child();
    writer.set_local_matrix(point_light_node, Mat4::translate((5., 10., 2.)));
    PointLightDataView {
      intensity: Vec3::new(1., 1., 1.) * 1.,
      cutoff_distance: 40.,
      node: point_light_node,
      scene: writer.scene,
    }
    .write(&mut writer.point_light_writer);
  }

  {
    let spot_light_node = writer.create_root_child();
    writer.set_local_matrix(spot_light_node, Mat4::translate((2., 2., 2.)));
    SpotLightDataView {
      intensity: Vec3::new(1., 0., 0.) * 180.,
      cutoff_distance: 40.,
      half_cone_angle: Deg::by(5. / 2.).to_rad(),
      half_penumbra_angle: Deg::by(5. / 2.).to_rad(),
      node: spot_light_node,
      scene: writer.scene,
    }
    .write(&mut writer.spot_light_writer);
  }

  // stress_test2(scene);
}

#[allow(dead_code)]
pub fn load_stress_test(scene: &mut SceneWriter) {
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
        let mesh = scene.write_attribute_mesh(mesh.build()).mesh;

        scene.create_scene_model(material, mesh, node);
      }
    }
  }
}

fn create_gpu_texture_by_fn(
  size: Size,
  pixel: impl Fn(usize, usize) -> Vec4<f32>,
) -> GPUBufferImage {
  let mut data: Vec<u8> = vec![0; size.area() * 4];
  let s = size.into_usize();
  for y in 0..s.1 {
    for x in 0..s.0 {
      let pixel = pixel(x, y);
      data[(y * s.0 + x) * 4] = (255.).min(pixel.x * 255.) as u8;
      data[(y * s.0 + x) * 4 + 1] = (255.).min(pixel.y * 255.) as u8;
      data[(y * s.0 + x) * 4 + 2] = (255.).min(pixel.z * 255.) as u8;
      data[(y * s.0 + x) * 4 + 3] = (255.).min(pixel.w * 255.) as u8;
    }
  }

  GPUBufferImage {
    data,
    format: TextureFormat::Rgba8UnormSrgb,
    size,
  }
}

pub fn textured_example_tex(scene: &mut SceneWriter) -> Texture2DWithSamplingDataView {
  let width = 256;

  // https://lodev.org/cgtutor/xortexture.html
  let tex = create_gpu_texture_by_fn(Size::from_u32_pair_min_one((width, width)), |x, y| {
    let c = (x as u8) ^ (y as u8);
    let r = 255 - c;
    let g = c;
    let b = c % 128;

    fn channel(c: u8) -> f32 {
      c as f32 / 255.
    }

    Vec4::new(channel(r), channel(g), channel(b), 1.)
  });

  scene
    .texture_sample_pair_writer()
    .write_tex_with_default_sampler(tex)
}

pub fn load_example_cube_tex(writer: &mut SceneWriter) -> EntityHandle<SceneTextureCubeEntity> {
  // use rendiation_texture_loader::load_tex;
  // let path = if cfg!(windows) {
  //   [
  //     "C:/Users/mk/Desktop/rrf-resource/Park2/posx.jpg",
  //     "C:/Users/mk/Desktop/rrf-resource/Park2/negx.jpg",
  //     "C:/Users/mk/Desktop/rrf-resource/Park2/posy.jpg",
  //     "C:/Users/mk/Desktop/rrf-resource/Park2/negy.jpg",
  //     "C:/Users/mk/Desktop/rrf-resource/Park2/posz.jpg",
  //     "C:/Users/mk/Desktop/rrf-resource/Park2/negz.jpg",
  //   ]
  // } else {
  //   [
  //     "/Users/mikialex/dev/references/three.js/examples/textures/cube/pisa/px.png",
  //     "/Users/mikialex/dev/references/three.js/examples/textures/cube/pisa/nx.png",
  //     "/Users/mikialex/dev/references/three.js/examples/textures/cube/pisa/py.png",
  //     "/Users/mikialex/dev/references/three.js/examples/textures/cube/pisa/ny.png",
  //     "/Users/mikialex/dev/references/three.js/examples/textures/cube/pisa/pz.png",
  //     "/Users/mikialex/dev/references/three.js/examples/textures/cube/pisa/nz.png",
  //   ]
  // };
  // let x_pos = load_tex(path[0]);
  // let y_pos = load_tex(path[2]);
  // let z_pos = load_tex(path[4]);
  // let x_neg = load_tex(path[1]);
  // let y_neg = load_tex(path[3]);
  // let z_neg = load_tex(path[5]);

  let width = 256;
  // simple grid texture with gradient background
  let tex = create_gpu_texture_by_fn(Size::from_u32_pair_min_one((width, width)), |x, y| {
    let u = x as f32 / width as f32;
    let v = y as f32 / width as f32;

    if x % 25 == 0 || y % 25 == 0 {
      return Vec4::new(0., 0., 0., 1.);
    }

    Vec4::new(0., u, v, 1.)
  });

  writer.cube_texture_writer().write_cube_tex(
    tex.clone(),
    tex.clone(),
    tex.clone(),
    tex.clone(),
    tex.clone(),
    tex.clone(),
  )
}

#[allow(dead_code)]
pub fn test_ltc_lighting(writer: &mut SceneWriter) {
  // ground
  {
    let mesh = build_attributes_mesh(|builder| {
      builder.triangulate_parametric(
        &ParametricPlane.transform_by(Mat4::scale((20., 20., 20.))),
        TessellationConfig { u: 1, v: 1 },
        true,
      );
    })
    .build();
    let attribute_mesh = writer.write_attribute_mesh(mesh).mesh;

    let material = PhysicalMetallicRoughnessMaterialDataView {
      base_color: Vec3::splat(1.),
      roughness: 0.5,
      ..Default::default()
    }
    .write(&mut writer.pbr_mr_mat_writer);
    let material = SceneMaterialDataView::PbrMRMaterial(material);

    let child = writer.create_root_child();
    writer.set_local_matrix(child, Mat4::rotate_x(-f32::PI() / 2.));

    writer.create_scene_model(material, attribute_mesh, child);
  }

  let area_light_writer = global_entity_of::<AreaLightEntity>().entity_writer();
  let node = writer.create_root_child();
  writer.set_local_matrix(node, Mat4::translate((10., 4., -10.)));

  area_light_writer
    .with_component_value_writer::<AreaLightRefNode>(node.some_handle())
    .with_component_value_writer::<AreaLightRefScene>(writer.scene.some_handle())
    .with_component_value_writer::<AreaLightIsRound>(true)
    .with_component_value_writer::<AreaLightIsDoubleSide>(false)
    .with_component_value_writer::<AreaLightIntensity>(Vec3::splat(100.))
    .with_component_value_writer::<AreaLightSize>(Vec2::new(1., 1.))
    .new_entity();
}
