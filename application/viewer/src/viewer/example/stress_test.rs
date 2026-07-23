use rand::Rng;
use rendiation_mesh_generator::*;

use crate::*;

const ROTATION_SPEED: f64 = 0.01;

pub fn use_stress_test_example(cx: &mut ViewerCx) {
  let (cx, example) = cx.use_state_init(|_| StressLoadUnloadExample::new());

  if let ViewerCxStage::SceneContentUpdate { writer, .. } = &mut cx.stage {
    let scene = cx.default_scene.scene;

    if example.pending_unload_all {
      for block in example.blocks.drain(..) {
        block.destroy(writer);
      }
      example.pending_unload_all = false;
    }

    if !example.pending_block_deletions.is_empty() {
      example.pending_block_deletions.sort_unstable();
      example.pending_block_deletions.dedup();
      for idx in example.pending_block_deletions.iter().rev() {
        if *idx < example.blocks.len() {
          let block = example.blocks.remove(*idx);
          block.destroy(writer);
        }
      }
      example.pending_block_deletions.clear();
    }

    for _ in 0..example.pending_loads {
      let block = StressBlock::new(
        writer,
        scene,
        example.use_unique_material,
        example.use_unique_mesh,
        example.h_count,
      );
      example.blocks.push(block);
    }
    example.pending_loads = 0;

    let mut rng = rand::rng();
    for block in &mut example.blocks {
      if block.rotate {
        block.angle += ROTATION_SPEED;
        let rotated = Mat4::rotate_y(block.angle);
        writer.set_local_matrix(block.root_node, rotated);
      }
      if block.animate_material {
        for &mat in &block.material_entities {
          let color = Vec3::new(
            rng.random::<f32>(),
            rng.random::<f32>(),
            rng.random::<f32>(),
          );
          writer
            .pbr_sg_mat_writer
            .write::<PbrSGMaterialAlbedoComponent>(mat, color);
        }
      }
    }
  }

  if let ViewerCxStage::Gui { egui_ctx, .. } = &mut cx.stage {
    egui::Window::new("Stress Test")
      .default_size((360., 400.))
      .vscroll(true)
      .show(egui_ctx, |ui| {
        ui.checkbox(&mut example.use_unique_material, "use unique material");
        ui.checkbox(&mut example.use_unique_mesh, "use unique mesh");

        let h_count_options = [1u32, 10, 50, 100];
        egui::ComboBox::from_label("h_count")
          .selected_text(format!("{}", example.h_count))
          .show_ui(ui, |ui| {
            for &opt in &h_count_options {
              ui.selectable_value(&mut example.h_count, opt, format!("{}", opt));
            }
          });

        ui.separator();

        ui.horizontal(|ui| {
          if ui.button("Load").clicked() {
            example.pending_loads += 1;
          }
          if !example.blocks.is_empty() {
            if ui.button("Unload All").clicked() {
              example.pending_unload_all = true;
            }
          }
        });

        ui.separator();
        ui.heading(format!("blocks ({})", example.blocks.len()));

        let mut to_delete = Vec::new();
        for (idx, block) in example.blocks.iter_mut().enumerate() {
          ui.group(|ui| {
            ui.horizontal(|ui| {
              ui.strong(format!(
                "#{}  {} models, h={}, {}mat, {}mesh",
                idx,
                block.model_count(),
                block.h_count,
                if block.has_unique_material {
                  "uni"
                } else {
                  "1"
                },
                if block.has_unique_mesh { "uni" } else { "1" },
              ));
              if ui.button("🗑").clicked() {
                to_delete.push(idx);
              }
            });
            ui.checkbox(&mut block.rotate, "rotate");
            ui.checkbox(&mut block.animate_material, "animate material");
          });
        }
        for idx in to_delete.into_iter().rev() {
          example.pending_block_deletions.push(idx);
        }
      });
  }
}

impl CanCleanUpFrom<ViewerDropCx<'_>> for StressLoadUnloadExample {
  fn drop_from_cx(&mut self, cx: &mut ViewerDropCx) {
    for block in self.blocks.drain(..) {
      block.destroy(&mut cx.writer);
    }
  }
}

struct StressLoadUnloadExample {
  blocks: Vec<StressBlock>,
  use_unique_material: bool,
  use_unique_mesh: bool,
  h_count: u32,
  pending_loads: u32,
  pending_unload_all: bool,
  pending_block_deletions: Vec<usize>,
}

impl StressLoadUnloadExample {
  fn new() -> Self {
    Self {
      blocks: Vec::new(),
      use_unique_material: false,
      use_unique_mesh: false,
      h_count: 10,
      pending_loads: 0,
      pending_unload_all: false,
      pending_block_deletions: Vec::new(),
    }
  }
}

struct StressBlock {
  leaf_units: Vec<LeafUnit>,
  i_nodes: Vec<EntityHandle<SceneNodeEntity>>,
  intermediate_nodes: Vec<EntityHandle<SceneNodeEntity>>,
  root_node: EntityHandle<SceneNodeEntity>,
  mesh_entities: Vec<AttributesMeshEntities>,
  material_entities: Vec<EntityHandle<PbrSGMaterialEntity>>,
  h_count: usize,
  has_unique_material: bool,
  has_unique_mesh: bool,
  rotate: bool,
  angle: f64,
  animate_material: bool,
}

struct LeafUnit {
  std_model: EntityHandle<StandardModelEntity>,
  scene_model: EntityHandle<SceneModelEntity>,
  node: EntityHandle<SceneNodeEntity>,
}

impl LeafUnit {
  fn destroy(self, writer: &mut SceneWriter) {
    writer.model_writer.delete_entity(self.scene_model);
    writer.std_model_writer.delete_entity(self.std_model);
    writer.node_writer.delete_entity(self.node);
  }
}

impl StressBlock {
  fn new(
    writer: &mut SceneWriter,
    scene: EntityHandle<SceneEntity>,
    use_unique_material: bool,
    use_unique_mesh: bool,
    h_count: u32,
  ) -> Self {
    let h_count = h_count as usize;

    let shared_material = create_mat(writer);

    let shared_mesh_entities;
    let shared_mesh;

    if use_unique_mesh {
      shared_mesh_entities = Vec::new();
      shared_mesh = None;
    } else {
      let cube = CubeMeshParameter {
        width: 0.2,
        height: 0.2,
        depth: 0.2,
      };
      let attribute_mesh = build_attributes_mesh(|builder| {
        for face in cube.make_faces() {
          builder.triangulate_parametric(&face, TessellationConfig { u: 2, v: 3 }, true);
        }
      })
      .build();
      let entities = writer.write_attribute_mesh(attribute_mesh);
      shared_mesh = Some(entities.mesh);
      shared_mesh_entities = vec![entities];
    }

    let node_count = 1 + 100 + 100 * 100 + 100 * 100 * h_count;
    let model_count = 100 * 100 * h_count;
    writer.node_writer.notify_reserve_changes(node_count);
    writer.std_model_writer.notify_reserve_changes(model_count);
    writer.model_writer.notify_reserve_changes(model_count);

    let root_node = writer.create_root_child();
    writer.set_local_matrix(root_node, Mat4::identity());

    let mut i_nodes = Vec::with_capacity(100);
    let mut intermediate_nodes = Vec::with_capacity(100 * 100);
    let mut leaf_units = Vec::with_capacity(100 * 100 * h_count);
    let mut material_entities = Vec::new();
    let mut mesh_entities = shared_mesh_entities;

    for i in 0..100 {
      let i_parent = writer.create_child(root_node);
      writer.set_local_matrix(i_parent, Mat4::translate((i as f64, 0., 0.)));
      i_nodes.push(i_parent);

      for j in 0..100 {
        let j_parent = writer.create_child(i_parent);
        writer.set_local_matrix(j_parent, Mat4::translate((0., 0., j as f64)));
        intermediate_nodes.push(j_parent);

        for k in 0..h_count {
          let node = writer.create_child(j_parent);
          writer.set_local_matrix(node, Mat4::translate((0., k as f64, 0.)));

          let material = if use_unique_material {
            let mat = create_mat(writer);
            if let SceneMaterialDataView::PbrSGMaterial(h) = mat {
              material_entities.push(h);
            }
            mat
          } else {
            shared_material
          };

          let mesh = if use_unique_mesh {
            let cube = CubeMeshParameter {
              width: 0.2,
              height: 0.2,
              depth: 0.2,
            };
            let attribute_mesh = build_attributes_mesh(|builder| {
              for face in cube.make_faces() {
                builder.triangulate_parametric(&face, TessellationConfig { u: 2, v: 3 }, true);
              }
            })
            .build();
            let entities = writer.write_attribute_mesh(attribute_mesh);
            mesh_entities.push(entities);
            mesh_entities.last().unwrap().mesh
          } else {
            shared_mesh.unwrap()
          };

          let (std_model, scene_model) = writer.create_scene_model(material, mesh, node, scene);
          leaf_units.push(LeafUnit {
            std_model,
            scene_model,
            node,
          });
        }
      }
    }

    if !use_unique_material {
      if let SceneMaterialDataView::PbrSGMaterial(h) = shared_material {
        material_entities.push(h);
      }
    }

    Self {
      leaf_units,
      i_nodes,
      intermediate_nodes,
      root_node,
      mesh_entities,
      material_entities,
      h_count,
      has_unique_material: use_unique_material,
      has_unique_mesh: use_unique_mesh,
      rotate: false,
      angle: 0.0,
      animate_material: false,
    }
  }

  fn destroy(self, writer: &mut SceneWriter) {
    for unit in self.leaf_units {
      unit.destroy(writer);
    }
    for node in self.i_nodes {
      writer.node_writer.delete_entity(node);
    }
    for node in self.intermediate_nodes {
      writer.node_writer.delete_entity(node);
    }
    writer.node_writer.delete_entity(self.root_node);
    for mat in self.material_entities {
      writer.pbr_sg_mat_writer.delete_entity(mat);
    }
    for entities in self.mesh_entities {
      entities.clean_up(&mut writer.mesh_writer, &mut writer.buffer_writer);
    }
  }

  fn model_count(&self) -> usize {
    self.leaf_units.len()
  }
}

fn create_mat(writer: &mut SceneWriter) -> SceneMaterialDataView {
  let material = PhysicalSpecularGlossinessMaterialDataView {
    albedo: Vec3::splat(1.),
    albedo_texture: None,
    ..Default::default()
  }
  .write(&mut writer.pbr_sg_mat_writer);
  SceneMaterialDataView::PbrSGMaterial(material)
}
