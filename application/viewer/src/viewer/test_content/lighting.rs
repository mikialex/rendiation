use rendiation_area_lighting::*;
use rendiation_mesh_generator::*;

use crate::*;

pub fn load_default_scene_lighting_test(writer: &mut SceneWriter) {
  {
    let directional_light_node = writer.create_root_child();
    writer.set_local_matrix(
      directional_light_node,
      Mat4::lookat(Vec3::splat(300.), Vec3::splat(0.), UP),
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
      Mat4::lookat(Vec3::new(30., 300., -30.), Vec3::splat(0.), UP),
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
      intensity: Vec3::new(1., 1., 1.) * 100.,
      cutoff_distance: 40.,
      node: point_light_node,
      scene: writer.scene,
    }
    .write(&mut writer.point_light_writer);
  }

  {
    let spot_light_node = writer.create_root_child();
    let spot_lookat = Mat4::lookat(Vec3::new(5., 5., -5.), Vec3::splat(0.), UP);
    writer.set_local_matrix(spot_light_node, spot_lookat);
    SpotLightDataView {
      intensity: Vec3::new(1., 0., 0.) * 1800.,
      cutoff_distance: 10.,
      half_cone_angle: Deg::by(60. / 2.).to_rad(),
      half_penumbra_angle: Deg::by(50. / 2.).to_rad(),
      node: spot_light_node,
      scene: writer.scene,
    }
    .write(&mut writer.spot_light_writer);
  }
}

pub fn load_ltc_lighting_test(writer: &mut SceneWriter) {
  // ground
  {
    let mesh = build_attributes_mesh(|builder| {
      builder.triangulate_parametric(
        &ParametricPlane.transform3d_by(Mat4::scale((20., 20., 20.))),
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
