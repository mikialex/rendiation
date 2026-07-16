use rendiation_mesh_generator::*;

use crate::*;

pub fn load_stress_test(
  writer: &mut SceneWriter,
  scene: EntityHandle<SceneEntity>,
  use_unique_material: bool,
) {
  let material = create_mat(writer);

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
  let mesh = writer.write_attribute_mesh(mesh.build()).mesh;

  let h_count = 50;
  let width_count = 100;
  let node_count = width_count + width_count * width_count + width_count * width_count * h_count;
  let model_count = width_count * width_count * h_count;
  writer.node_writer.notify_reserve_changes(node_count);
  writer.std_model_writer.notify_reserve_changes(model_count);
  writer.model_writer.notify_reserve_changes(model_count);

  for i in 0..width_count {
    let i_parent = writer.create_root_child();
    writer.set_local_matrix(i_parent, Mat4::translate((i as f64, 0., 0.)));
    for j in 0..width_count {
      let j_parent = writer.create_child(i_parent);
      writer.set_local_matrix(j_parent, Mat4::translate((0., 0., j as f64)));
      for k in 0..h_count {
        let node = writer.create_child(j_parent);
        writer.set_local_matrix(node, Mat4::translate((0., k as f64, 0.)));

        let material = if use_unique_material {
          create_mat(writer)
        } else {
          material
        };

        writer.create_scene_model(material, mesh, node, scene);
      }
    }
  }
}

fn create_mat(s_writer: &mut SceneWriter) -> SceneMaterialDataView {
  let material = PhysicalSpecularGlossinessMaterialDataView {
    albedo: Vec3::splat(1.),
    albedo_texture: None,
    ..Default::default()
  }
  .write(&mut s_writer.pbr_sg_mat_writer);
  SceneMaterialDataView::PbrSGMaterial(material)
}
