use rendiation_mesh_core::{AttributeIndexFormat, AttributesMeshData, CommonVertex};
use rendiation_mesh_simplification::*;

use crate::*;

pub fn use_mesh_tools(cx: &mut ViewerCx) {
  let (cx, simp_req) = cx.use_plain_state::<Option<SimplifySelectMeshRequest>>();

  if let ViewerCxStage::Gui { egui_ctx, global } = &mut cx.stage {
    let opened = global.features.entry("mesh tools").or_default();

    egui::Window::new("Mesh Tools")
      .open(opened)
      .vscroll(true)
      .show(egui_ctx, |ui| {
        if cx.viewer.scene.selected_target.is_some() {
          if ui.button("simplify selected mesh").clicked() {
            *simp_req = Some(SimplifySelectMeshRequest(None));
          }
        } else {
          ui.label("pick a target to view available mesh tool options");
        }
      });
  }

  if let ViewerCxStage::EventHandling { reader, .. } = &mut cx.stage {
    if let Some(simp_req) = simp_req {
      if let Some(target) = cx.viewer.scene.selected_target {
        let mesh = get_mesh(reader, target);

        let mut dest_idx = vec![0; mesh.indices.len()];

        let config = EdgeCollapseConfig {
          target_index_count: mesh.indices.len() / 2,
          target_error: f32::INFINITY,
          lock_border: false,
        };

        let EdgeCollapseResult {
          result_error,
          result_count,
        } = simplify_by_edge_collapse(&mut dest_idx, &mesh.indices, &mesh.vertices, None, config);
        println!("{result_error}, {result_count}");

        dest_idx.resize(result_count, 0);

        let mesh = MeshBufferSource {
          vertices: mesh.vertices,
          indices: dest_idx,
        }
        .remap_vertex();

        simp_req.0 = Some(mesh);
      }
    }
  }

  if let ViewerCxStage::SceneContentUpdate { writer, .. } = &mut cx.stage {
    if let Some(SimplifySelectMeshRequest(Some(mesh))) = simp_req.take() {
      create_simplified_mesh(writer, cx.viewer.scene.selected_target.unwrap(), mesh);
    }
  }
}

struct SimplifySelectMeshRequest(Option<MeshBufferSource>);

fn get_mesh(reader: &SceneReader, target: EntityHandle<SceneModelEntity>) -> MeshBufferSource {
  let std_model = reader.read_scene_model(target).model;
  let mesh = reader.read_std_model(std_model).mesh;
  let mesh = reader.read_attribute_mesh(mesh);

  let (fmt, indices) = mesh.indices.clone().unwrap();
  assert!(fmt == rendiation_mesh_core::AttributeIndexFormat::Uint32);

  let mesh = mesh.read_full();
  let position = mesh
    .get_attribute(&rendiation_mesh_core::AttributeSemantic::Positions)
    .unwrap();
  let normals = mesh
    .get_attribute(&rendiation_mesh_core::AttributeSemantic::Normals)
    .unwrap();
  let uvs = mesh
    .get_attribute(&rendiation_mesh_core::AttributeSemantic::TexCoords(0))
    .unwrap();

  let position = position.visit_slice::<Vec3<f32>>().unwrap();
  let normals = normals.visit_slice::<Vec3<f32>>().unwrap();
  let uvs = uvs.visit_slice::<Vec2<f32>>().unwrap();

  let vertices = position
    .iter()
    .zip(normals.iter())
    .zip(uvs.iter())
    .map(|((&position, &normal), &uv)| CommonVertex {
      position,
      normal,
      uv,
    })
    .collect::<Vec<_>>();

  MeshBufferSource {
    indices: indices.read().visit_slice().unwrap().to_vec(),
    vertices,
  }
}

fn create_simplified_mesh(
  writer: &mut SceneWriter,
  target: EntityHandle<SceneModelEntity>,
  mesh: MeshBufferSource,
) {
  let attribute_mesh = AttributesMeshData {
    attributes: vec![
      (
        rendiation_mesh_core::AttributeSemantic::Positions,
        mesh
          .vertices
          .iter()
          .flat_map(|v| v.position.bytes().iter().copied())
          .collect(),
      ),
      (
        rendiation_mesh_core::AttributeSemantic::Normals,
        mesh
          .vertices
          .iter()
          .flat_map(|v| v.normal.bytes().iter().copied())
          .collect(),
      ),
      (
        rendiation_mesh_core::AttributeSemantic::TexCoords(0),
        mesh
          .vertices
          .iter()
          .flat_map(|v| v.uv.bytes().iter().copied())
          .collect(),
      ),
    ],
    indices: Some((
      AttributeIndexFormat::Uint32,
      mesh
        .indices
        .iter()
        .flat_map(|v| v.bytes().iter().copied())
        .collect(),
    )),
    mode: rendiation_mesh_core::PrimitiveTopology::TriangleList,
    groups: Default::default(),
  }
  .build();

  let attribute_mesh = writer.write_attribute_mesh(attribute_mesh).mesh;

  let std_model = writer
    .model_writer
    .read_foreign_key::<SceneModelStdModelRenderPayload>(target)
    .unwrap();
  let std_model = writer.std_model_writer.clone_entity(std_model);
  writer
    .std_model_writer
    .write_foreign_key::<StandardModelRefAttributesMeshEntity>(std_model, attribute_mesh.into());

  let child = writer.create_root_child();

  SceneModelDataView {
    model: std_model,
    scene: writer.scene,
    node: child,
  }
  .write(&mut writer.model_writer);
}
