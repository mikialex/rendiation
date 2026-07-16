use super::util::SceneModelWithUniqueNode;
use crate::*;

pub fn use_cell_mesh_example(cx: &mut ViewerCx) {
  let (cx, example) = cx.use_state_init(|_| CellMeshExample::new());

  if let ViewerCxStage::SceneContentUpdate { writer, .. } = &mut cx.stage {
    // initialize default lights once
    if example.lights.is_none() {
      example.lights = Some(CommonTestLights::new(writer, cx.default_scene.scene));
    }

    let mut cell_mesh_writer = global_entity_of::<CellMeshEntity>().entity_writer();

    // process shrink ratio updates
    for inst in &mut example.instances {
      if inst.shrink_ratio_dirty {
        cell_mesh_writer.write::<CellMeshShrinkRatio>(inst.cell_mesh, inst.shrink_ratio);
        inst.shrink_ratio_dirty = false;
      }
    }

    // process deletions
    if !example.pending_deletions.is_empty() {
      while let Some(inst) = example.pending_deletions.pop() {
        inst.destroy(writer, &mut cell_mesh_writer);
      }
    }

    // process additions
    if !example.pending_additions.is_empty() {
      while example.pending_additions.pop().is_some() {
        example.create_instance(writer, &mut cell_mesh_writer, cx.default_scene.scene);
      }
    }
  }

  if let ViewerCxStage::Gui { egui_ctx, .. } = &mut cx.stage {
    egui::Window::new("Cell Mesh (FEM Visualization)")
      .default_size((360., 520.))
      .vscroll(true)
      .show(egui_ctx, |ui| {
        ui.heading("Cell Mesh Example");
        ui.label("Simple FEM cell mesh visualization.");
        ui.label("Each cell is a quad with shrink-to-center effect.");

        ui.separator();

        ui.horizontal(|ui| {
          if ui.button("add cell mesh").clicked() {
            example.pending_additions.push(());
          }
          if ui.button("clear all").clicked() {
            example
              .pending_deletions
              .extend(example.instances.drain(..));
          }
        });

        ui.separator();
        ui.heading(format!("instances ({})", example.instances.len()));
        egui::ScrollArea::vertical()
          .max_height(350.)
          .show(ui, |ui| {
            let mut to_remove = Vec::new();
            for (idx, inst) in example.instances.iter_mut().enumerate() {
              ui.group(|ui| {
                ui.horizontal(|ui| {
                  ui.label(format!("#{}", idx));
                  if ui.button("🗑").clicked() {
                    to_remove.push(idx);
                  }
                });
                let changed =
                  ui.add(egui::Slider::new(&mut inst.shrink_ratio, 0.0..=1.0).text("shrink"));
                if changed.changed() {
                  inst.shrink_ratio_dirty = true;
                }
              });
            }
            for idx in to_remove.into_iter().rev() {
              let inst = example.instances.remove(idx);
              example.pending_deletions.push(inst);
            }
          });
      });
  }
}

struct CellMeshExample {
  instances: Vec<CellMeshInstance>,
  pending_additions: Vec<()>,
  pending_deletions: Vec<CellMeshInstance>,
  lights: Option<CommonTestLights>,
}

impl CanCleanUpFrom<ViewerDropCx<'_>> for CellMeshExample {
  fn drop_from_cx(&mut self, cx: &mut ViewerDropCx) {
    let mut cell_mesh_writer = global_entity_of::<CellMeshEntity>().entity_writer();
    for inst in self.instances.drain(..) {
      inst.destroy(&mut cx.writer, &mut cell_mesh_writer);
    }
    for inst in self.pending_deletions.drain(..) {
      inst.destroy(&mut cx.writer, &mut cell_mesh_writer);
    }
    if let Some(lights) = self.lights.take() {
      lights.destroy(&mut cx.writer);
    }
  }
}

impl CellMeshExample {
  pub fn new() -> Self {
    Self {
      instances: Vec::new(),
      pending_additions: Vec::new(),
      pending_deletions: Vec::new(),
      lights: None,
    }
  }

  fn create_instance(
    &mut self,
    writer: &mut SceneWriter,
    cell_mesh_writer: &mut EntityWriter<CellMeshEntity>,
    scene: EntityHandle<SceneEntity>,
  ) {
    let mesh_data = make_default_fem_mesh();
    let shrink_ratio = 0.8;

    let cell_mesh = cell_mesh_writer.new_entity(|w| {
      w.write::<CellMeshUnitsBuffer>(&ExternalRefPtr::new(mesh_data))
        .write::<CellMeshShrinkRatio>(&shrink_ratio)
    });

    let material = PhysicalMetallicRoughnessMaterialDataView {
      base_color: Vec3::new(0.2, 0.6, 0.9),
      roughness: 0.3,
      metallic: 0.1,
      ..Default::default()
    }
    .write(&mut writer.pbr_mr_mat_writer);

    let node = writer.create_root_child();
    writer.set_local_matrix(node, Mat4::identity());

    let scene = scene.some_handle();
    let std_model = writer.std_model_writer.new_entity(|w| {
      w.write::<StandardModelRefPbrMRMaterial>(&material.some_handle())
        .write::<StandardModelCellMeshPayload>(&cell_mesh.some_handle())
    });

    let model = writer.model_writer.new_entity(|w| {
      w.write::<SceneModelStdModelRenderPayload>(&std_model.some_handle())
        .write::<SceneModelBelongsToScene>(&scene)
        .write::<SceneModelRefNode>(&node.some_handle())
    });

    self.instances.push(CellMeshInstance {
      cell_mesh,
      std_model,
      scene_unit: SceneModelWithUniqueNode { model, node },
      material,
      shrink_ratio,
      shrink_ratio_dirty: false,
    });
  }
}

struct CellMeshInstance {
  cell_mesh: EntityHandle<CellMeshEntity>,
  std_model: EntityHandle<StandardModelEntity>,
  scene_unit: SceneModelWithUniqueNode,
  material: EntityHandle<PbrMRMaterialEntity>,
  shrink_ratio: f32,
  shrink_ratio_dirty: bool,
}

impl CellMeshInstance {
  fn destroy(self, writer: &mut SceneWriter, cell_mesh_writer: &mut EntityWriter<CellMeshEntity>) {
    writer.model_writer.delete_entity(self.scene_unit.model);
    writer.node_writer.delete_entity(self.scene_unit.node);
    writer.std_model_writer.delete_entity(self.std_model);
    cell_mesh_writer.delete_entity(self.cell_mesh);
    writer.pbr_mr_mat_writer.delete_entity(self.material);
  }
}

/// Build a simple 3x3 FEM cell mesh as a flat grid with slight curvature.
///
/// Each cell is a quad with a distinct color based on position for FEM data visualization.
fn make_default_fem_mesh() -> Vec<CellMeshUnitData> {
  let mut cells = Vec::new();
  let grid_size = 3;
  let cell_size = 1.0;
  let offset = -(grid_size as f32 * cell_size) / 2.0;

  for i in 0..grid_size {
    for j in 0..grid_size {
      let x0 = offset + i as f32 * cell_size;
      let z0 = offset + j as f32 * cell_size;
      let x1 = x0 + cell_size;
      let z1 = z0 + cell_size;

      // slight Y displacement for curvature
      let y00 = 0.1 * ((x0 * x0 + z0 * z0) * 0.3).sin();
      let y10 = 0.1 * ((x1 * x1 + z0 * z0) * 0.3).sin();
      let y11 = 0.1 * ((x1 * x1 + z1 * z1) * 0.3).sin();
      let y01 = 0.1 * ((x0 * x0 + z1 * z1) * 0.3).sin();

      let p1 = Vec3::new(x0, y00, z0);
      let p2 = Vec3::new(x1, y10, z0);
      let p3 = Vec3::new(x1, y11, z1);
      let p4 = Vec3::new(x0, y01, z1);
      let center = (p1 + p2 + p3 + p4) / 4.0;

      // color gradient based on grid position for FEM data visualization
      let t = (i + j) as f32 / ((grid_size - 1) * 2) as f32;
      let front_color = Vec3::new(0.2 + t * 0.6, 0.3 + t * 0.5, 0.8 - t * 0.4);
      let back_color = front_color * 0.5;

      cells.push(CellMeshUnitData {
        p1,
        p2,
        p3,
        p4,
        center,
        front_face_color: front_color,
        back_face_color: back_color,
      });
    }
  }

  cells
}
