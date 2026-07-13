use rendiation_mesh_segmentation::*;
use rendiation_mesh_simplification::*;

use crate::{viewer::use_scene_reader, *};

pub fn use_mesh_tools(cx: &mut ViewerCx) {
  let (cx, simp_req) = cx.use_plain_state::<Option<SimplifySelectMeshRequest>>();
  let (cx, seg_req) = cx.use_plain_state::<Option<MeshSegmentationDebugRequest>>();

  if let ViewerCxStage::Gui {
    egui_ctx, global, ..
  } = &mut cx.stage
  {
    let opened = global.features.entry("mesh tools").or_insert(false);

    egui::Window::new("Mesh Tools")
      .open(opened)
      .default_size((100., 100.))
      .vscroll(true)
      .show(egui_ctx, |ui| {
        if cx.viewer.selection.selected_model.if_single().is_some() {
          if ui.button("simplification edge collapse").clicked() {
            *simp_req = Some(SimplifySelectMeshRequest(
              None,
              MeshToolSimplificationType::EdgeCollapse,
            ));
          }
          if ui.button("simplification sloppy").clicked() {
            *simp_req = Some(SimplifySelectMeshRequest(
              None,
              MeshToolSimplificationType::Sloppy,
            ));
          }
          if ui.button("segmentation").clicked() {
            *seg_req = Some(MeshSegmentationDebugRequest(None));
          }
        } else {
          ui.label("pick a target to view available mesh tool options");
        }
      });
  }

  let reader = use_scene_reader(cx);

  if let ViewerCxStage::EventHandling { .. } = &mut cx.stage {
    let reader = &reader.unwrap();
    if let Some(simp_req) = simp_req {
      if let Some(target) = cx.viewer.selection.selected_model.if_single() {
        if let Some(mesh) = get_mesh(reader, target) {
          let mut dest_idx = vec![0; mesh.indices.len()];

          let SimplificationResult {
            result_error,
            result_count,
          } = match simp_req.1 {
            MeshToolSimplificationType::EdgeCollapse => {
              let config = EdgeCollapseConfig {
                target_index_count: mesh.indices.len() / 2,
                target_error: f32::INFINITY,
                lock_border: false,
                use_absolute_error: true,
              };

              simplify_by_edge_collapse(&mut dest_idx, &mesh.indices, &mesh.vertices, None, config)
            }
            MeshToolSimplificationType::Sloppy => simplify_sloppy(
              &mut dest_idx,
              &mesh.indices,
              &mesh.vertices,
              None,
              mesh.indices.len() as u32 / 2,
              f32::INFINITY,
              true,
            ),
          };

          println!("result_error: {result_error}, result_index_count: {result_count}");

          dest_idx.resize(result_count, 0);

          let mesh = CommonMeshBuffer {
            vertices: mesh.vertices,
            indices: dest_idx,
          }
          .deduplicate_indices_and_remove_unused_vertices();

          if mesh.indices.is_empty() {
            println!("mesh is simplified to nothing, this may be a bug");
          } else {
            simp_req.0 = Some(mesh);
          }
        }
      }
    }

    if let Some(req) = seg_req {
      if let Some(target) = cx.viewer.selection.selected_model.if_single() {
        if let Some(mesh) = get_mesh(reader, target) {
          req.0 = Some(mesh_segmentation_debug(mesh));
        }
      }
    }
  }

  if let ViewerCxStage::SceneContentUpdate { writer, .. } = &mut cx.stage {
    let scene = cx.active_surface_content.scene;
    if let Some(SimplifySelectMeshRequest(Some(mesh), _)) = simp_req.take() {
      if let Some(target) = cx.viewer.selection.selected_model.if_single() {
        create_simplified_mesh(writer, scene, target, mesh);
      }
    }

    if let Some(MeshSegmentationDebugRequest(Some(meshes))) = seg_req.take() {
      meshes.into_iter().for_each(|mesh| {
        create_segmented_debug_mesh(writer, scene, mesh);
      });
    }
  }
}

pub enum MeshToolSimplificationType {
  EdgeCollapse,
  Sloppy,
}

struct SimplifySelectMeshRequest(Option<CommonMeshBuffer>, MeshToolSimplificationType);

struct MeshSegmentationDebugRequest(Option<Vec<CommonMeshBuffer>>);

fn mesh_segmentation_debug(mesh: CommonMeshBuffer) -> Vec<CommonMeshBuffer> {
  let config = ClusteringConfig {
    max_vertices: 64,
    max_triangles: 124, // NVidia-recommended 126, rounded down to a multiple of 4
    cone_weight: 0.0,
  };

  let max_meshlets = build_meshlets_bound(mesh.indices.len(), &config);
  let mut meshlets = vec![rendiation_mesh_segmentation::Meshlet::default(); max_meshlets];

  let mut meshlet_vertices = vec![0; max_meshlets * config.max_vertices as usize];
  let mut meshlet_triangles = vec![0; max_meshlets * config.max_triangles as usize * 3];

  let count = build_meshlets::<_, rendiation_mesh_segmentation::BVHSpaceSearchAcceleration>(
    &config,
    &mesh.indices,
    &mesh.vertices,
    &mut meshlets,
    &mut meshlet_vertices,
    &mut meshlet_triangles,
  );

  meshlets
    .get(0..count as usize)
    .unwrap()
    .iter()
    .map(|meshlet| {
      let tri_range = meshlet.triangle_offset as usize
        ..(meshlet.triangle_offset + meshlet.triangle_count * 3) as usize;
      let offset = meshlet.vertex_offset as usize;
      let tri = meshlet_triangles.get(tri_range).unwrap();

      let vertices = tri
        .iter()
        .map(|i| meshlet_vertices[offset + *i as usize])
        .map(|i| mesh.vertices[i as usize]);

      let (indices, vertices) = create_deduplicated_index_vertex_mesh(vertices);
      CommonMeshBuffer { indices, vertices }
    })
    .collect()
}

fn get_mesh(
  reader: &SceneReader,
  target: EntityHandle<SceneModelEntity>,
) -> Option<CommonMeshBuffer> {
  let std_model = reader.try_read_scene_model(target);
  if std_model.is_none() {
    log::warn!("not s std mesh");
  }
  let std_model = std_model?.model;
  let mesh = reader.read_std_model(std_model).mesh;
  let mesh = reader
    .read_attribute_mesh(mesh)
    .into_living()?
    .into_attributes_mesh();

  let (fmt, indices) = mesh.indices.clone().unwrap();
  assert!(fmt == AttributeIndexFormat::Uint32);

  let position = mesh.get_attribute(&AttributeSemantic::Positions).unwrap();
  let normals = mesh.get_attribute(&AttributeSemantic::Normals).unwrap();
  let uvs = mesh
    .get_attribute(&AttributeSemantic::TexCoords(0))
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

  CommonMeshBuffer {
    indices: indices.visit_slice().unwrap().to_vec(),
    vertices,
  }
  .into()
}

fn create_mesh(
  writer: &mut SceneWriter,
  mesh: CommonMeshBuffer,
) -> EntityHandle<AttributesMeshEntity> {
  let attribute_mesh = AttributesMeshData {
    attributes: vec![
      (
        AttributeSemantic::Positions,
        mesh
          .vertices
          .iter()
          .flat_map(|v| bytemuck::bytes_of(&v.position).iter().copied())
          .collect(),
      ),
      (
        AttributeSemantic::Normals,
        mesh
          .vertices
          .iter()
          .flat_map(|v| bytemuck::bytes_of(&v.normal).iter().copied())
          .collect(),
      ),
      (
        AttributeSemantic::TexCoords(0),
        mesh
          .vertices
          .iter()
          .flat_map(|v| bytemuck::bytes_of(&v.uv).iter().copied())
          .collect(),
      ),
    ],
    indices: Some((
      AttributeIndexFormat::Uint32,
      mesh
        .indices
        .iter()
        .flat_map(|v| bytemuck::bytes_of(v).iter().copied())
        .collect(),
    )),
    mode: MeshPrimitiveTopology::TriangleList,
  }
  .build();

  writer.write_attribute_mesh(attribute_mesh).mesh
}

fn create_segmented_debug_mesh(
  writer: &mut SceneWriter,
  scene: EntityHandle<SceneEntity>,
  mesh: CommonMeshBuffer,
) {
  let mesh = create_mesh(writer, mesh);

  let r: f32 = rand::random();
  let g: f32 = rand::random();
  let b: f32 = rand::random();

  let material = UnlitMaterialDataView {
    color: Vec4::new(r, g, b, 1.),
    ..Default::default()
  }
  .write(&mut writer.unlit_mat_writer);
  let material = SceneMaterialDataView::UnlitMaterial(material);

  let child = writer.create_root_child();
  writer.create_scene_model(material, mesh, child, scene);
}

fn create_simplified_mesh(
  writer: &mut SceneWriter,
  scene: EntityHandle<SceneEntity>,
  target: EntityHandle<SceneModelEntity>,
  mesh: CommonMeshBuffer,
) {
  let mesh = create_mesh(writer, mesh);
  let std_model = writer
    .model_writer
    .read_foreign_key::<SceneModelStdModelRenderPayload>(target)
    .unwrap();
  let std_model = writer.std_model_writer.clone_entity(std_model);
  writer
    .std_model_writer
    .write_foreign_key::<StandardModelRefAttributesMeshEntity>(std_model, mesh.into());

  let child = writer.create_root_child();

  SceneModelDataView {
    model: std_model,
    scene,
    node: child,
  }
  .write(&mut writer.model_writer);
}
