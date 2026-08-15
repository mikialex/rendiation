use rendiation_mesh_generator::*;

use super::util::{DirectionalLightWithNode, PointLightWithNode, SceneModelWithUniqueNode};
use crate::*;

pub fn use_point_light_shadow_example(cx: &mut ViewerCx) {
  let (cx, example) = cx.use_state_init(|_| PointLightShadowExample::new());

  if let ViewerCxStage::SceneContentUpdate { writer, .. } = &mut cx.stage {
    if !example.initialized {
      example.create_scene(writer, cx.default_scene.scene);
    }

    example.update_light(writer, cx.absolute_seconds_from_start);
  }

  if let ViewerCxStage::Gui { egui_ui, .. } = &mut cx.stage {
    egui::Window::new("Point Light Shadow")
      .default_size((340., 200.))
      .show(egui_ui, |ui| {
        ui.heading("Point Light Shadow Example");
        ui.label("A point light orbits inside a cube with one face removed.");
        ui.label("Two static spheres cast omni-directional shadows onto the cube walls.");
        ui.separator();
        ui.label("Toggle shadow filters (PCF/VSM) in the lighting panel.");
      });
  }
}

struct PointLightShadowExample {
  initialized: bool,
  scene_models: Vec<SceneModelWithUniqueNode>,
  std_models: Vec<EntityHandle<StandardModelEntity>>,
  mesh_entities: Vec<AttributesMeshEntities>,
  materials: Vec<EntityHandle<PbrMRMaterialEntity>>,
  point_light: Option<PointLightWithNode>,
  key_light: Option<DirectionalLightWithNode>,
}

impl CanCleanUpFrom<ViewerDropCx<'_>> for PointLightShadowExample {
  fn drop_from_cx(&mut self, cx: &mut ViewerDropCx) {
    for std_model in self.std_models.drain(..) {
      cx.writer.std_model_writer.delete_entity(std_model);
    }
    for mesh_entities in self.mesh_entities.drain(..) {
      mesh_entities.clean_up(&mut cx.writer.mesh_writer, &mut cx.writer.buffer_writer);
    }
    for material in self.materials.drain(..) {
      cx.writer.pbr_mr_mat_writer.delete_entity(material);
    }
    for scene_model in self.scene_models.drain(..) {
      scene_model.destroy(&mut cx.writer);
    }
    if let Some(light) = self.point_light.take() {
      light.destroy(&mut cx.writer);
    }
    if let Some(light) = self.key_light.take() {
      light.destroy(&mut cx.writer);
    }
  }
}

impl PointLightShadowExample {
  pub fn new() -> Self {
    Self {
      initialized: false,
      scene_models: Vec::new(),
      std_models: Vec::new(),
      mesh_entities: Vec::new(),
      materials: Vec::new(),
      point_light: None,
      key_light: None,
    }
  }

  fn create_scene(&mut self, writer: &mut SceneWriter, scene: EntityHandle<SceneEntity>) {
    self.initialized = true;

    // the cube is missing its +z face so the interior is visible
    let cube_param = CubeMeshParameter {
      width: 8.,
      height: 8.,
      depth: 8.,
    };
    let cube_mesh = build_attributes_mesh(|builder| {
      for (face, surface) in cube_param.make_faces().into_iter().enumerate() {
        if face == 4 {
          continue;
        }
        builder.triangulate_parametric(&surface, TessellationConfig { u: 1, v: 1 }, true);
      }
    })
    .build();
    self.add_model(
      writer,
      scene,
      cube_mesh,
      Vec3::new(0.7, 0.7, 0.75),
      0.9,
      0.,
      true,
    );

    let sphere_mesh = |radius: f32| {
      build_attributes_mesh(|builder| {
        builder.triangulate_parametric(
          &UVSphere.transform3d_by(Mat4::scale(Vec3::splat(radius))),
          TessellationConfig { u: 32, v: 16 },
          true,
        );
      })
      .build()
    };

    self.add_model_at(
      writer,
      scene,
      sphere_mesh(1.2),
      Vec3::new(-2., -2.8, -1.),
      Vec3::new(0.85, 0.25, 0.2),
      0.4,
      0.1,
      false,
    );
    self.add_model_at(
      writer,
      scene,
      sphere_mesh(0.8),
      Vec3::new(2., -3.2, 1.6),
      Vec3::new(0.2, 0.45, 0.85),
      0.5,
      0.,
      false,
    );

    // a weak key light so the outside of the cube is not pitch black
    let key_light_node = writer.create_root_child();
    writer.set_local_matrix(
      key_light_node,
      Mat4::lookat(Vec3::new(20., 30., 20.), Vec3::splat(0.), UP),
    );
    let key_light = DirectionalLightDataView {
      illuminance: Vec3::splat(0.5),
      node: key_light_node,
      scene,
    }
    .write(&mut writer.directional_light_writer);
    self.key_light = Some(DirectionalLightWithNode {
      entity: key_light,
      node: key_light_node,
    });

    let point_light_node = writer.create_root_child();
    writer.set_local_matrix(point_light_node, Mat4::identity());
    let point_light = PointLightDataView {
      intensity: Vec3::splat(120.),
      cutoff_distance: 12.,
      node: point_light_node,
      scene,
    }
    .write(&mut writer.point_light_writer);

    // the default 256 face resolution is a bit coarse for the large cube walls
    writer
      .point_light_writer
      .write::<BasicShadowMapResolutionOf<PointLightBasicShadowInfo>>(
        point_light,
        Vec2::new(512, 512),
      );

    self.point_light = Some(PointLightWithNode {
      entity: point_light,
      node: point_light_node,
    });
  }

  fn add_model(
    &mut self,
    writer: &mut SceneWriter,
    scene: EntityHandle<SceneEntity>,
    mesh: AttributesMesh,
    color: Vec3<f32>,
    roughness: f32,
    metallic: f32,
    double_sided: bool,
  ) {
    self.add_model_at(
      writer,
      scene,
      mesh,
      Vec3::splat(0.),
      color,
      roughness,
      metallic,
      double_sided,
    );
  }

  fn add_model_at(
    &mut self,
    writer: &mut SceneWriter,
    scene: EntityHandle<SceneEntity>,
    mesh: AttributesMesh,
    position: Vec3<f32>,
    color: Vec3<f32>,
    roughness: f32,
    metallic: f32,
    double_sided: bool,
  ) {
    let material = PhysicalMetallicRoughnessMaterialDataView {
      base_color: color,
      roughness,
      metallic,
      ..Default::default()
    }
    .write(&mut writer.pbr_mr_mat_writer);

    let node = writer.create_root_child();
    writer.set_local_matrix(node, Mat4::translate(position.into_f64()));

    let mesh_entities = writer.write_solid_attribute_mesh(mesh);

    let std_model = if double_sided {
      let states_override = RasterizationStates {
        cull_mode: None,
        ..Default::default()
      };

      StandardModelDataView::new(
        SceneMaterialDataView::PbrMRMaterial(material),
        mesh_entities.mesh,
      )
      .with_states_override(states_override)
      .write(&mut writer.std_model_writer)
    } else {
      StandardModelDataView::new(
        SceneMaterialDataView::PbrMRMaterial(material),
        mesh_entities.mesh,
      )
      .write(&mut writer.std_model_writer)
    };

    let model = SceneModelDataView {
      model: std_model,
      scene,
      node,
    }
    .write(&mut writer.model_writer);

    self.materials.push(material);
    self.mesh_entities.push(mesh_entities);
    self.std_models.push(std_model);
    self
      .scene_models
      .push(SceneModelWithUniqueNode { model, node });
  }

  fn update_light(&self, writer: &mut SceneWriter, time: f32) {
    let t = time * 0.7;
    let orbit_radius = 2.6;
    let position = Vec3::new(
      t.cos() * orbit_radius,
      1.0 + (t * 2.3).sin() * 0.8,
      t.sin() * orbit_radius,
    );

    if let Some(light) = &self.point_light {
      writer.set_local_matrix(light.node, Mat4::translate(position.into_f64()));
    }
  }
}
