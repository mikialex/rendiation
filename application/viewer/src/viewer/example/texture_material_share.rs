use rand::Rng;
use rendiation_mesh_generator::*;

use super::util::{CommonTestLights, SceneModelWithUniqueNode};
use crate::*;

/// Test scene demonstrating texture and material sharing across multiple material types.
///
/// Scene layout (8 balls, 4 rows x 2 columns):
///   Row 1-2 (y=0, y=-2): 4 PbrMR balls sharing 2 material instances (2 balls per material)
///   Row 3   (y=-4):      2 PbrSG balls, each with its own material instance
///   Row 4   (y=-6):      2 OccStyle balls sharing one OccStyleEffectControlEntity
///
/// All 8 balls share the same texture entity.
/// When the texture is replaced, all 8 balls update regardless of material type.
///
/// Controls:
///   - "Replace Shared Texture" — creates a new random XOR texture; checkbox controls
///     whether to modify the existing texture entity in-place or create/delete entities
///   - "Toggle Material 0 Alpha" — switches the first PbrMR material between
///     alpha 0.5 (Blend) and 1.0 (Opaque), affecting its 2 balls
pub fn use_texture_material_share_example(cx: &mut ViewerCx) {
  let (cx, example) = cx.use_state_init(|_| TextureAndMaterialShareExample::new());

  if let ViewerCxStage::SceneContentUpdate { writer, .. } = &mut cx.stage {
    if !example.initialized {
      example.initialize(writer, cx.default_scene.scene);
    }

    if example.replace_texture_pending {
      example.replace_texture(writer);
      example.replace_texture_pending = false;
    }

    if example.toggle_alpha_pending {
      example.toggle_alpha(writer);
      example.toggle_alpha_pending = false;
    }
  }

  if let ViewerCxStage::Gui { egui_ui, .. } = &mut cx.stage {
    egui::Window::new("Texture and Material Share")
      .default_size((400., 100.))
      .show(egui_ui, |ui| {
        ui.heading("Eight Balls Sharing One Texture, Multiple Material Types");
        if ui.button("Replace Shared Texture").clicked() {
          example.replace_texture_pending = true;
        }
        ui.checkbox(
          &mut example.modify_texture_directly,
          "Modify texture content directly (no create/delete entity)",
        );
        if ui.button("Toggle Material 0 Alpha (0.5 / 1.0)").clicked() {
          example.toggle_alpha_pending = true;
        }
      });
  }
}

impl CanCleanUpFrom<ViewerDropCx<'_>> for TextureAndMaterialShareExample {
  fn drop_from_cx(&mut self, cx: &mut ViewerDropCx) {
    for unit in self.scene_units.drain(..) {
      unit.destroy(&mut cx.writer);
    }
    for mat in self.mr_materials.drain(..) {
      cx.writer.pbr_mr_mat_writer.delete_entity(mat);
    }
    for mat in self.sg_materials.drain(..) {
      cx.writer.pbr_sg_mat_writer.delete_entity(mat);
    }
    {
      let mut occ_writer = global_entity_of::<OccStyleMaterialEntity>().entity_writer();
      for mat in self.occ_materials.drain(..) {
        occ_writer.delete_entity(mat);
      }
    }
    if let Some(effect) = self.occ_effect.take() {
      global_entity_of::<OccStyleEffectControlEntity>()
        .entity_writer()
        .delete_entity(effect);
    }
    if let Some(tex) = self.shared_texture.take() {
      cx.writer.tex_writer.delete_entity(tex.texture);
      cx.writer.sampler_writer.delete_entity(tex.sampler);
    }
    if let Some(lights) = self.lights.take() {
      lights.destroy(&mut cx.writer);
    }
  }
}

struct TextureAndMaterialShareExample {
  mr_materials: Vec<EntityHandle<PbrMRMaterialEntity>>,
  sg_materials: Vec<EntityHandle<PbrSGMaterialEntity>>,
  occ_materials: Vec<EntityHandle<OccStyleMaterialEntity>>,
  occ_effect: Option<EntityHandle<OccStyleEffectControlEntity>>,
  scene_units: Vec<SceneModelWithUniqueNode>,
  shared_texture: Option<Texture2DWithSamplingDataView>,
  lights: Option<CommonTestLights>,
  replace_texture_pending: bool,
  toggle_alpha_pending: bool,
  alpha_is_transparent: bool,
  modify_texture_directly: bool,
  initialized: bool,
}

impl TextureAndMaterialShareExample {
  fn new() -> Self {
    Self {
      mr_materials: Vec::new(),
      sg_materials: Vec::new(),
      occ_materials: Vec::new(),
      occ_effect: None,
      scene_units: Vec::new(),
      shared_texture: None,
      lights: None,
      replace_texture_pending: false,
      toggle_alpha_pending: false,
      alpha_is_transparent: false,
      modify_texture_directly: false,
      initialized: false,
    }
  }

  fn initialize(&mut self, writer: &mut SceneWriter, scene: EntityHandle<SceneEntity>) {
    // Build a shared sphere mesh used by all 8 balls
    let attribute_mesh = build_attributes_mesh(|builder| {
      builder.triangulate_parametric(
        &SphereMeshParameter::default().make_surface(),
        TessellationConfig { u: 16, v: 16 },
        true,
      );
    })
    .build();
    let mesh = writer.write_solid_attribute_mesh(attribute_mesh).mesh;

    // Create a single texture entity shared by all materials across all 8 balls
    let texture = Self::create_xor_texture(writer);
    self.shared_texture = Some(texture);

    // --- Materials ---

    // 2 PbrMR materials, both referencing the shared texture
    for _ in 0..2 {
      let material = PhysicalMetallicRoughnessMaterialDataView {
        base_color: Vec3::splat(0.8),
        base_color_texture: Some(texture),
        roughness: 0.1,
        metallic: 0.8,
        ..Default::default()
      }
      .write(&mut writer.pbr_mr_mat_writer);
      self.mr_materials.push(material);
    }

    // 2 PbrSG materials, both referencing the shared texture
    for _ in 0..2 {
      let material = PhysicalSpecularGlossinessMaterialDataView {
        albedo: Vec3::splat(1.0),
        albedo_texture: Some(texture),
        ..Default::default()
      }
      .write(&mut writer.pbr_sg_mat_writer);
      self.sg_materials.push(material);
    }

    // 2 OccStyle materials referencing the shared texture, plus a shared effect
    {
      let mut occ_writer = global_entity_of::<OccStyleMaterialEntity>().entity_writer();
      for _ in 0..2 {
        let occ_material = occ_writer.new_entity(|w| {
          let w = w
            .write::<OccStyleMaterialDiffuse>(&Vec4::new(0.8, 0.8, 0.8, 1.0))
            .write::<OccStyleMaterialSpecular>(&Vec3::new(1.0, 1.0, 1.0))
            .write::<OccStyleMaterialShininess>(&200.)
            .write::<OccStyleMaterialEmissive>(&Vec3::zero());
          texture.write::<OccStyleMaterialDiffuseTex>(w)
        });
        self.occ_materials.push(occ_material);
      }

      let effect = global_entity_of::<OccStyleEffectControlEntity>()
        .entity_writer()
        .new_entity(|w| w.write::<OccStyleEffectShadeType>(&OccStyleEffectType::Lighted));
      self.occ_effect = Some(effect);

      for &occ_material in &self.occ_materials {
        occ_writer.write::<OccStyleMaterialEffect>(occ_material, effect.some_handle());
      }
    }

    // --- Scene models (4 rows x 2 columns) ---

    // Row 1-2: 4 PbrMR balls, each pair reusing one of the 2 material instances
    let mr_positions = [
      (-1.5, 0.0, 0.0),
      (1.5, 0.0, 0.0),
      (-1.5, -2.0, 0.0),
      (1.5, -2.0, 0.0),
    ];
    for (i, &pos) in mr_positions.iter().enumerate() {
      let node = writer.create_root_child();
      writer.set_local_matrix(node, Mat4::translate(pos));
      let model = writer.create_scene_model(
        SceneMaterialDataView::PbrMRMaterial(self.mr_materials[i % 2]),
        mesh,
        node,
        scene,
      );
      self
        .scene_units
        .push(SceneModelWithUniqueNode { model, node });
    }

    // Row 3: 2 PbrSG balls, each with its own material instance
    let sg_positions = [(-1.5, -4.0, 0.0), (1.5, -4.0, 0.0)];
    for (i, &pos) in sg_positions.iter().enumerate() {
      let node = writer.create_root_child();
      writer.set_local_matrix(node, Mat4::translate(pos));
      let model = writer.create_scene_model(
        SceneMaterialDataView::PbrSGMaterial(self.sg_materials[i]),
        mesh,
        node,
        scene,
      );
      self
        .scene_units
        .push(SceneModelWithUniqueNode { model, node });
    }

    // Row 4: 2 OccStyle balls, manually wired via StandardModel + SceneModel entities
    // (OccStyle uses StdModelOccStyleMaterialPayload instead of SceneMaterialDataView)
    let occ_positions = [(-1.5, -6.0, 0.0), (1.5, -6.0, 0.0)];
    // safe: SceneWriter always has a target scene when called from ViewerCx stage
    for (i, &pos) in occ_positions.iter().enumerate() {
      let node = writer.create_root_child();
      writer.set_local_matrix(node, Mat4::translate(pos));
      let std_model = writer.std_model_writer.new_entity(|w| {
        w.write::<StandardModelRefAttributesMeshEntity>(&mesh.some_handle())
          .write::<StdModelOccStyleMaterialPayload>(&self.occ_materials[i].some_handle())
      });
      let model = writer.model_writer.new_entity(|w| {
        w.write::<SceneModelStdModelRenderPayload>(&std_model.some_handle())
          .write::<SceneModelBelongsToScene>(&scene.some_handle())
          .write::<SceneModelRefNode>(&node.some_handle())
      });
      self
        .scene_units
        .push(SceneModelWithUniqueNode { model, node });
    }

    self.lights = Some(CommonTestLights::new(writer, scene));
    self.initialized = true;
  }

  fn create_xor_texture(writer: &mut SceneWriter) -> Texture2DWithSamplingDataView {
    let width = 256;
    let tex = create_gpu_texture_by_fn(Size::from_u32_pair_min_one((width, width)), |x, y| {
      let c = (x as u8) ^ (y as u8);
      let r = 255 - c;
      let g = c;
      let b = c % 128;

      fn channel(c: u8) -> f32 {
        c as f32 / 255.
      }

      Vec4::new(channel(r), channel(g), channel(b), 1.)
    });

    writer
      .texture_sample_pair_writer()
      .write_direct_tex_with_default_sampler(tex)
  }

  fn random_texture_data() -> GPUBufferImage {
    let mut rng = rand::rng();
    let r_shift: u8 = rng.random();
    let g_shift: u8 = rng.random();
    let b_shift: u8 = rng.random();

    let width = 256;
    create_gpu_texture_by_fn(Size::from_u32_pair_min_one((width, width)), |x, y| {
      let c = (x as u8) ^ (y as u8);
      let r = 255 - c.wrapping_add(r_shift);
      let g = c.wrapping_add(g_shift);
      let b = (c.wrapping_add(b_shift)) % 128;

      fn channel(c: u8) -> f32 {
        c as f32 / 255.
      }

      Vec4::new(channel(r), channel(g), channel(b), 1.)
    })
  }

  fn create_random_texture(writer: &mut SceneWriter) -> Texture2DWithSamplingDataView {
    let tex = Self::random_texture_data();
    writer
      .texture_sample_pair_writer()
      .write_direct_tex_with_default_sampler(tex)
  }

  fn replace_texture(&mut self, writer: &mut SceneWriter) {
    // Direct path: modify the existing texture entity's content in-place.
    // All FK references stay valid; every material sees the new content immediately.
    if self.modify_texture_directly {
      let tex_data = Arc::new(Self::random_texture_data());
      let content = Some(ExternalRefPtr::new(MaybeUriData::Living(tex_data)));
      // safe: initialize() created shared_texture before any replace_texture call
      let tex = self.shared_texture.expect("shared texture not initialized");
      writer
        .tex_writer
        .write::<SceneTexture2dEntityDirectContent>(tex.texture, content);
      return;
    }

    // Indirect path: create a new texture+sampler pair, update all material FK refs,
    // then delete the old texture+sampler entities.
    let new_texture = Self::create_random_texture(writer);

    for &material in &self.mr_materials {
      writer
        .pbr_mr_mat_writer
        .write::<SceneTexture2dRefOf<PbrMRMaterialBaseColorAlphaTex>>(
          material,
          new_texture.texture.some_handle(),
        );
      writer
        .pbr_mr_mat_writer
        .write::<SceneSamplerRefOf<PbrMRMaterialBaseColorAlphaTex>>(
          material,
          new_texture.sampler.some_handle(),
        );
    }

    for &material in &self.sg_materials {
      writer
        .pbr_sg_mat_writer
        .write::<SceneTexture2dRefOf<PbrSGMaterialAlbedoAlphaTex>>(
          material,
          new_texture.texture.some_handle(),
        );
      writer
        .pbr_sg_mat_writer
        .write::<SceneSamplerRefOf<PbrSGMaterialAlbedoAlphaTex>>(
          material,
          new_texture.sampler.some_handle(),
        );
    }

    {
      let mut occ_writer = global_entity_of::<OccStyleMaterialEntity>().entity_writer();
      for &material in &self.occ_materials {
        occ_writer
          .write::<SceneTexture2dRefOf<OccStyleMaterialDiffuseTex>>(
            material,
            new_texture.texture.some_handle(),
          )
          .write::<SceneSamplerRefOf<OccStyleMaterialDiffuseTex>>(
            material,
            new_texture.sampler.some_handle(),
          );
      }
    }

    // safe: shared_texture holds the old tex+sampler that all materials just stopped referencing
    if let Some(old) = self.shared_texture.replace(new_texture) {
      writer.tex_writer.delete_entity(old.texture);
      writer.sampler_writer.delete_entity(old.sampler);
    }
  }

  fn toggle_alpha(&mut self, writer: &mut SceneWriter) {
    // safe: initialize() creates 2 mr_materials, toggle_alpha only runs after initialize
    let material = self.mr_materials[0];
    self.alpha_is_transparent = !self.alpha_is_transparent;
    if self.alpha_is_transparent {
      writer
        .pbr_mr_mat_writer
        .write::<AlphaOf<PbrMRMaterialAlphaConfig>>(material, 0.5);
      writer
        .pbr_mr_mat_writer
        .write::<AlphaModeOf<PbrMRMaterialAlphaConfig>>(material, AlphaMode::Blend);
    } else {
      writer
        .pbr_mr_mat_writer
        .write::<AlphaOf<PbrMRMaterialAlphaConfig>>(material, 1.0);
      writer
        .pbr_mr_mat_writer
        .write::<AlphaModeOf<PbrMRMaterialAlphaConfig>>(material, AlphaMode::Opaque);
    }
  }
}
