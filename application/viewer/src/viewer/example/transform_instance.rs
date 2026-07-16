use rand::Rng;
use rendiation_mesh_generator::*;

use super::util::{CommonTestLights, SceneModelWithUniqueNode};
use crate::*;

#[derive(Clone, Copy, PartialEq)]
enum GroupType {
  WideLine,
  Cube,
}

pub fn use_transform_instance_example(cx: &mut ViewerCx) {
  let (cx, example) = cx.use_state_init(|_| TransformInstanceExample::new());

  if let ViewerCxStage::SceneContentUpdate { writer, .. } = &mut cx.stage {
    let scene = cx.default_scene.scene;
    if !example.initialized {
      example.initialize(writer, scene);
    }

    // Process group-level deletions
    example.pending_group_deletions.sort_unstable();
    example.pending_group_deletions.dedup();
    for idx in example.pending_group_deletions.iter().rev() {
      if *idx < example.groups.len() {
        let group = example.groups.remove(*idx);
        global_entity_of::<TransformInstancedModelEntity>()
          .entity_writer()
          .delete_entity(group.instance_entity);
        group.instance_scene_model.destroy(writer);
      }
    }
    example.pending_group_deletions.clear();

    // Process new group creation
    for _ in 0..example.pending_new_wideline_groups {
      example.create_group(writer, scene, GroupType::WideLine);
    }
    example.pending_new_wideline_groups = 0;
    for _ in 0..example.pending_new_cube_groups {
      example.create_group(writer, scene, GroupType::Cube);
    }
    example.pending_new_cube_groups = 0;

    // Process instance-level changes per group
    for group in &mut example.groups {
      let mut transform_instanced_writer =
        global_entity_of::<TransformInstancedModelEntity>().entity_writer();

      // Deletions — remove from highest index downward, keep at least 1
      if !group.pending_deletions.is_empty() {
        group.pending_deletions.sort_unstable();
        group.pending_deletions.dedup();
        group.pending_deletions.reverse();

        let max_remove = group.instance_transforms.len() - 1;
        let actual_remove = group.pending_deletions.len().min(max_remove);
        for i in 0..actual_remove {
          let idx = group.pending_deletions[i];
          if idx < group.instance_transforms.len() {
            group.instance_transforms.remove(idx);
          }
        }
        group.pending_deletions.clear();
        group.dirty = true;
      }

      // Additions — random transforms near origin
      if group.pending_additions > 0 {
        let mut rng = rand::rng();
        for _ in 0..group.pending_additions {
          let x: f32 = rng.random_range(-3.0..3.0);
          let y: f32 = rng.random_range(-3.0..3.0);
          let z: f32 = rng.random_range(-3.0..3.0);
          group.instance_transforms.push(Mat4::translate((x, y, z)));
        }
        group.pending_additions = 0;
        group.dirty = true;
      }

      // Flush GPU buffer
      if group.dirty {
        let buffer = ExternalRefPtr::new(group.instance_transforms.clone());
        transform_instanced_writer
          .write::<TransformInstancedModelInstanceBuffer>(group.instance_entity, buffer);
        group.dirty = false;
      }
    }
  }

  if let ViewerCxStage::Gui { egui_ctx, .. } = &mut cx.stage {
    egui::Window::new("Transform Instance Example")
      .default_size((420., 600.))
      .vscroll(true)
      .show(egui_ctx, |ui| {
        ui.heading("Instance Groups");

        ui.horizontal(|ui| {
          if ui.button("➕ new wide-line group").clicked() {
            example.pending_new_wideline_groups += 1;
          }
          if ui.button("➕ new cube group").clicked() {
            example.pending_new_cube_groups += 1;
          }
          if !example.groups.is_empty() {
            if ui.button("🗑 delete all groups").clicked() {
              example
                .pending_group_deletions
                .extend(0..example.groups.len());
            }
          }
        });

        ui.separator();

        // Collect commands during display; apply after iteration
        let mut group_to_delete = None;
        let mut add_in_group = None;
        let mut delete_in_group: Vec<(usize, usize)> = Vec::new();

        let group_count = example.groups.len();
        for group_idx in 0..group_count {
          let group = &example.groups[group_idx];
          let type_label = match group.group_type {
            GroupType::WideLine => "W",
            GroupType::Cube => "C",
          };

          ui.group(|ui| {
            ui.horizontal(|ui| {
              let label = format!(
                "[{type_label}] Group #{group_idx}  ({} instance{})",
                group.instance_transforms.len(),
                if group.instance_transforms.len() > 1 {
                  "s"
                } else {
                  ""
                },
              );
              ui.strong(label);
              if ui.button("🗑 group").clicked() {
                group_to_delete = Some(group_idx);
              }
            });

            for (inst_idx, _mat) in group.instance_transforms.iter().enumerate() {
              ui.horizontal(|ui| {
                ui.label(format!("#{inst_idx}"));
                let can_delete = group.instance_transforms.len() > 1;
                if ui.add_enabled(can_delete, egui::Button::new("🗑")).clicked() {
                  delete_in_group.push((group_idx, inst_idx));
                }
                if !can_delete {
                  ui.weak("(last one, can't delete)");
                }
              });
            }

            if ui.button("add instance").clicked() {
              add_in_group = Some(group_idx);
            }
          });
          ui.add_space(4.);
        }

        // Apply commands
        if let Some(idx) = group_to_delete {
          example.pending_group_deletions.push(idx);
        }
        if let Some(idx) = add_in_group {
          example.groups[idx].pending_additions += 1;
        }
        for (g_idx, i_idx) in delete_in_group {
          example.groups[g_idx].pending_deletions.push(i_idx);
        }
      });
  }
}

impl CanCleanUpFrom<ViewerDropCx<'_>> for TransformInstanceExample {
  fn drop_from_cx(&mut self, cx: &mut ViewerDropCx) {
    for group in self.groups.drain(..) {
      global_entity_of::<TransformInstancedModelEntity>()
        .entity_writer()
        .delete_entity(group.instance_entity);
      group.instance_scene_model.destroy(&mut cx.writer);
    }
    if let Some(source) = self.source_wide_line.take() {
      global_entity_of::<WideLineModelEntity>()
        .entity_writer()
        .delete_entity(source.wide_line_entity);
      source.scene_model.destroy(&mut cx.writer);
    }
    if let Some(source) = self.source_cube.take() {
      writer_std_model(&mut cx.writer).delete_entity(source.std_model);
      source
        .mesh_entities
        .clean_up(&mut cx.writer.mesh_writer, &mut cx.writer.buffer_writer);
      cx.writer.pbr_sg_mat_writer.delete_entity(source.material);
      source.scene_model.destroy(&mut cx.writer);
    }
    if let Some(lights) = self.lights.take() {
      lights.destroy(&mut cx.writer);
    }
  }
}

fn writer_std_model(writer: &mut SceneWriter) -> &mut TableWriter<StandardModelEntity> {
  &mut writer.std_model_writer
}

struct InstanceGroup {
  group_type: GroupType,
  instance_transforms: Vec<Mat4<f32>>,
  instance_entity: EntityHandle<TransformInstancedModelEntity>,
  instance_scene_model: SceneModelWithUniqueNode,
  pending_additions: u32,
  pending_deletions: Vec<usize>,
  dirty: bool,
}

struct WideLineSource {
  wide_line_entity: EntityHandle<WideLineModelEntity>,
  scene_model: SceneModelWithUniqueNode,
}

struct CubeSource {
  std_model: EntityHandle<StandardModelEntity>,
  mesh_entities: AttributesMeshEntities,
  material: EntityHandle<PbrSGMaterialEntity>,
  scene_model: SceneModelWithUniqueNode,
}

struct TransformInstanceExample {
  groups: Vec<InstanceGroup>,
  pending_new_wideline_groups: u32,
  pending_new_cube_groups: u32,
  pending_group_deletions: Vec<usize>,

  source_wide_line: Option<WideLineSource>,
  source_cube: Option<CubeSource>,
  lights: Option<CommonTestLights>,
  initialized: bool,
}

impl TransformInstanceExample {
  fn new() -> Self {
    Self {
      groups: Vec::new(),
      pending_new_wideline_groups: 0,
      pending_new_cube_groups: 0,
      pending_group_deletions: Vec::new(),
      source_wide_line: None,
      source_cube: None,
      lights: None,
      initialized: false,
    }
  }

  fn initialize(&mut self, writer: &mut SceneWriter, scene: EntityHandle<SceneEntity>) {
    self.init_wide_line_source(writer, scene);
    self.init_cube_source(writer, scene);
    self.lights = Some(CommonTestLights::new(writer, scene));
    self.initialized = true;
  }

  fn init_wide_line_source(&mut self, writer: &mut SceneWriter, scene: EntityHandle<SceneEntity>) {
    let mesh_buffer = build_wide_line_mesh(|builder| {
      builder.build_grid_parametric(
        &SphereMeshParameter::default().make_surface(),
        TessellationConfig { u: 6, v: 6 },
        true,
      );
    });

    let mut wide_line_writer = global_entity_of::<WideLineModelEntity>().entity_writer();
    let wide_line_model = wide_line_writer.new_entity(|w| {
      w.write::<WideLineWidth>(&3.)
        .write::<WideLineStylePattern>(&0xffc0)
        .write::<WideLineStyleFactor>(&6.0)
        .write::<WideLineMeshBuffer>(&mesh_buffer)
    });

    let source_node = writer
      .node_writer
      .new_entity(|w| w.write::<SceneNodeVisibleComponent>(&false));
    writer.set_local_matrix(source_node, Mat4::identity());

    let scene = scene.some_handle();
    let source_scene_model = writer.model_writer.new_entity(|w| {
      w.write::<SceneModelWideLineRenderPayload>(&wide_line_model.some_handle())
        .write::<SceneModelBelongsToScene>(&scene)
        .write::<SceneModelRefNode>(&source_node.some_handle())
    });

    self.source_wide_line = Some(WideLineSource {
      wide_line_entity: wide_line_model,
      scene_model: SceneModelWithUniqueNode {
        model: source_scene_model,
        node: source_node,
      },
    });
  }

  fn init_cube_source(&mut self, writer: &mut SceneWriter, scene: EntityHandle<SceneEntity>) {
    let cube = CubeMeshParameter {
      width: 1.,
      height: 2.,
      depth: 3.,
    };
    let attribute_mesh = build_attributes_mesh_non_indexed(|builder| {
      for face in cube.make_faces() {
        builder.triangulate_parametric(&face, TessellationConfig { u: 2, v: 3 }, true);
      }
    })
    .build();

    let mesh_entities = writer.write_solid_attribute_mesh(attribute_mesh);

    let mat_handle = PhysicalSpecularGlossinessMaterialDataView {
      albedo: Vec3::splat(1.),
      ..Default::default()
    }
    .write(&mut writer.pbr_sg_mat_writer);

    let source_node = writer
      .node_writer
      .new_entity(|w| w.write::<SceneNodeVisibleComponent>(&false));
    writer.set_local_matrix(source_node, Mat4::identity());

    let (std_model, source_scene_model) = writer.create_scene_model(
      SceneMaterialDataView::PbrSGMaterial(mat_handle),
      mesh_entities.mesh,
      source_node,
      scene,
    );

    self.source_cube = Some(CubeSource {
      std_model,
      mesh_entities,
      material: mat_handle,
      scene_model: SceneModelWithUniqueNode {
        model: source_scene_model,
        node: source_node,
      },
    });
  }

  fn create_group(
    &mut self,
    writer: &mut SceneWriter,
    scene: EntityHandle<SceneEntity>,
    ty: GroupType,
  ) {
    let scene_model_ref_node = match ty {
      GroupType::WideLine => &self
        .source_wide_line
        .as_ref()
        .expect("wide-line source not initialized")
        .scene_model
        .model
        .some_handle(),
      GroupType::Cube => &self
        .source_cube
        .as_ref()
        .expect("cube source not initialized")
        .scene_model
        .model
        .some_handle(),
    };

    let mut rng = rand::rng();
    let (x, y, z): (f32, f32, f32) = (
      rng.random_range(-3.0..3.0),
      rng.random_range(-3.0..3.0),
      rng.random_range(-3.0..3.0),
    );
    let initial_transform = vec![Mat4::translate((x, y, z))];
    let buffer = ExternalRefPtr::new(initial_transform.clone());

    let mut transform_instanced_writer =
      global_entity_of::<TransformInstancedModelEntity>().entity_writer();
    let instance_entity = transform_instanced_writer.new_entity(|w| {
      w.write::<TransformInstancedModelInstanceBuffer>(&buffer)
        .write::<TransformInstancedModelRefSceneModel>(scene_model_ref_node)
    });

    let scene = scene.some_handle();
    let instanced_node = writer.create_root_child();
    writer.set_local_matrix(instanced_node, Mat4::translate((0., 0., 0.)));

    let instance_scene_model = writer.model_writer.new_entity(|w| {
      w.write::<SceneModelTransformInstancedModelPayload>(&instance_entity.some_handle())
        .write::<SceneModelBelongsToScene>(&scene)
        .write::<SceneModelRefNode>(&instanced_node.some_handle())
    });

    self.groups.push(InstanceGroup {
      group_type: ty,
      instance_transforms: initial_transform,
      instance_entity,
      instance_scene_model: SceneModelWithUniqueNode {
        model: instance_scene_model,
        node: instanced_node,
      },
      pending_additions: 0,
      pending_deletions: Vec::new(),
      dirty: false,
    });
  }
}
