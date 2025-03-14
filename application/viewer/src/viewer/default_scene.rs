use rendiation_algebra::*;
use rendiation_mesh_generator::*;

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
      Mat4::translate((2., 0., 3.)) * Mat4::scale((2., 1., 1.)),
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

  // add another camera for camera related helper testing
  {
    let camera_node = writer.create_root_child();
    writer.set_local_matrix(
      camera_node,
      Mat4::lookat(Vec3::splat(3.), Vec3::splat(0.), UP),
    );
    writer
      .camera_writer
      .component_value_writer::<SceneCameraPerspective>(Some(PerspectiveProjection::default()))
      .component_value_writer::<SceneCameraBelongsToScene>(Some(writer.scene.into_raw()))
      .component_value_writer::<SceneCameraNode>(Some(camera_node.into_raw()))
      .new_entity();
  }

  load_default_scene_lighting_test(writer);
}
