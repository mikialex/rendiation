use rendiation_mesh_generator::*;

use crate::*;

pub fn load_transparent_test(writer: &mut SceneWriter, root: EntityHandle<SceneNodeEntity>) {
  // create a plane mesh
  let plane = ParametricPlane;
  let plane = build_attributes_mesh(|builder| {
    builder.triangulate_parametric(&plane, TessellationConfig { u: 1, v: 1 }, true);
  })
  .build();
  let plane = writer.write_attribute_mesh(plane).mesh;

  let scale = Mat4::scale((2., 2., 2.));
  {
    let material = create_alpha_material(Vec3::new(1., 0., 0.), writer);
    let child = writer.create_child(root);
    writer.set_local_matrix(child, Mat4::translate((0., 0., 0.)) * scale);
    writer.create_scene_model(material, plane, child);

    let material = create_alpha_material(Vec3::new(0., 1., 0.), writer);
    let child = writer.create_child(root);
    writer.set_local_matrix(child, Mat4::translate((0., 0., 1.)) * scale);
    writer.create_scene_model(material, plane, child);

    let material = create_alpha_material(Vec3::new(0., 0., 1.), writer);
    let child = writer.create_child(root);
    writer.set_local_matrix(child, Mat4::translate((0., 0., -1.)) * scale);
    writer.create_scene_model(material, plane, child);
  }
}

fn create_alpha_material(color: Vec3<f32>, writer: &mut SceneWriter) -> SceneMaterialDataView {
  let material = PhysicalSpecularGlossinessMaterialDataView {
    albedo: color,
    alpha: AlphaConfigDataView {
      alpha_mode: AlphaMode::Blend,
      alpha: 0.5,
      ..Default::default()
    },
    ..Default::default()
  }
  .write(&mut writer.pbr_sg_mat_writer);
  SceneMaterialDataView::PbrSGMaterial(material)
}
