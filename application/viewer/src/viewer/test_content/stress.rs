use rendiation_mesh_generator::*;

use crate::*;

pub fn load_stress_test(scene: &mut SceneWriter) {
  let material = PhysicalSpecularGlossinessMaterialDataView {
    albedo: Vec3::splat(1.),
    albedo_texture: None,
    ..Default::default()
  }
  .write(&mut scene.pbr_sg_mat_writer);
  let material = SceneMaterialDataView::PbrSGMaterial(material);

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

  let h_count = 50;
  let node_count = 100 + 100 * 100 + 100 * 100 * h_count;
  let model_count = 100 * 100 * h_count;
  scene.node_writer.notify_reserve_changes(node_count);
  scene.std_model_writer.notify_reserve_changes(model_count);
  scene.model_writer.notify_reserve_changes(model_count);

  for i in 0..100 {
    let i_parent = scene.create_root_child();
    scene.set_local_matrix(i_parent, Mat4::translate((i as f64, 0., 0.)));
    for j in 0..100 {
      let j_parent = scene.create_child(i_parent);
      scene.set_local_matrix(j_parent, Mat4::translate((0., 0., j as f64)));
      for k in 0..h_count {
        let node = scene.create_child(j_parent);
        scene.set_local_matrix(node, Mat4::translate((0., k as f64, 0.)));

        scene.create_scene_model(material, mesh, node);
      }
    }
  }
}
