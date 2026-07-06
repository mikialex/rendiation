use crate::{viewer::feature::egui_view::db_view::EGUIDataView, *};

pub fn inspect_selected(
  ui: &mut egui::Ui,
  selection: &mut ViewerSelectionStates,
  scene: EntityHandle<SceneEntity>,
) -> Option<()> {
  let mut scene_writer = SceneWriter::from_global(scene);
  if let Some(target) = selection.selected_model.if_single() {
    ui.label(format!("SceneModel id: {:?}", target.into_raw()));
    show_entity_label(&scene_writer.model_writer, target, ui);

    ui.separator();
    let node = scene_writer
      .model_writer
      .read_foreign_key::<SceneModelRefNode>(target)
      .unwrap();

    ui.label(format!("referenced node id: {:?}", node.into_raw()));
    show_entity_label(&scene_writer.node_writer, node, ui);

    let parent = scene_writer.node_writer.read::<SceneNodeParentIdx>(node);
    ui.label(format!("parent node id: {:?}", parent));

    let local_mat = scene_writer
      .node_writer
      .read::<SceneNodeLocalMatrixComponent>(node);

    ui.label("local matrix:");
    local_mat.hover_detail_view(ui);

    ui.separator();

    if let Some(text3d) = scene_writer
      .model_writer
      .read_foreign_key::<SceneModelText3dPayload>(target)
    {
      let mut w = global_entity_of::<Text3dEntity>().entity_writer();
      let content = w.read::<Text3dContent>(text3d).unwrap();

      let mut c = (*content).clone();
      crate::viewer::example::text3d_content_edit_ui(ui, &mut c);

      if c != *content {
        w.write::<Text3dContent>(text3d, Some(ExternalRefPtr::new(c)));
      }
    }

    let std_model = scene_writer
      .model_writer
      .read_foreign_key::<SceneModelStdModelRenderPayload>(target)?;

    ui.label(format!(
      "referenced std_model id: {:?}",
      std_model.into_raw()
    ));
    show_entity_label(&scene_writer.std_model_writer, std_model, ui);

    ui.separator();

    if ui.button("change to unlit").clicked() {
      let new_unlit = scene_writer.unlit_mat_writer.new_entity(|w| w);
      scene_writer
        .std_model_writer
        .write_foreign_key::<StandardModelRefPbrMRMaterial>(std_model, None);
      scene_writer
        .std_model_writer
        .write_foreign_key::<StandardModelRefPbrSGMaterial>(std_model, None);
      scene_writer
        .std_model_writer
        .write_foreign_key::<StandardModelRefUnlitMaterial>(std_model, Some(new_unlit));
    }

    if let Some(mat) = scene_writer
      .std_model_writer
      .read_foreign_key::<StandardModelRefPbrMRMaterial>(std_model)
    {
      ui.label("pbr mr material");
      ui.label(format!("material id: {:?}", mat.into_raw()));
      show_entity_label(&scene_writer.pbr_mr_mat_writer, mat, ui);
      modify_color_like_com::<PbrMRMaterialBaseColorComponent>(
        ui,
        &mut scene_writer.pbr_mr_mat_writer,
        mat,
      );
      modify_normalized_value_like_com::<PbrMRMaterialRoughnessComponent>(
        ui,
        &mut scene_writer.pbr_mr_mat_writer,
        mat,
      );
      modify_normalized_value_like_com::<PbrMRMaterialMetallicComponent>(
        ui,
        &mut scene_writer.pbr_mr_mat_writer,
        mat,
      );

      //
    } else if let Some(mat) = scene_writer
      .std_model_writer
      .read_foreign_key::<StandardModelRefPbrSGMaterial>(std_model)
    {
      ui.label("pbr sg material");
      ui.label(format!("material id: {:?}", mat.into_raw()));
      show_entity_label(&scene_writer.pbr_sg_mat_writer, mat, ui);
      modify_color_like_com::<PbrSGMaterialAlbedoComponent>(
        ui,
        &mut scene_writer.pbr_sg_mat_writer,
        mat,
      );
      modify_normalized_value_like_com::<PbrSGMaterialGlossinessComponent>(
        ui,
        &mut scene_writer.pbr_sg_mat_writer,
        mat,
      );
      modify_color_like_com::<PbrSGMaterialSpecularComponent>(
        ui,
        &mut scene_writer.pbr_sg_mat_writer,
        mat,
      );
    } else if let Some(mat) = scene_writer
      .std_model_writer
      .read_foreign_key::<StandardModelRefUnlitMaterial>(std_model)
    {
      ui.label("unlit material");
      ui.label(format!("material id: {:?}", mat.into_raw()));
      show_entity_label(&scene_writer.unlit_mat_writer, mat, ui);
      //
    } else {
      ui.label("unknown material type");
    }

    //
  } else if let Some(target) = selection.selected_dir_light {
    let w = &mut scene_writer.directional_light_writer;
    ui.label(format!("Scene directional id: {:?}", target.into_raw()));

    show_entity_label(w, target, ui);
    modify_bool_com::<DirectionalLightEnabled>(ui, w, target, "enabled");

    modify_bool_com::<BasicShadowMapEnabledOf<DirectionLightBasicShadowInfo>>(
      ui,
      w,
      target,
      "shadow enabled",
    );
    //
  } else if let Some(target) = selection.selected_spot_light {
    let w = &mut scene_writer.spot_light_writer;
    ui.label(format!("Scene Spotlight id: {:?}", target.into_raw()));

    show_entity_label(w, target, ui);
    modify_bool_com::<SpotLightEnabled>(ui, w, target, "enabled");

    ui.label("spotlight half cone angle:");
    modify_ranged_value_like_slider_com::<SpotLightHalfConeAngle>(
      ui,
      w,
      target,
      0.0..=(f32::PI() / 4.0),
    );

    ui.label("spotlight penumbra angle:");
    modify_ranged_value_like_slider_com::<SpotLightHalfPenumbraAngle>(
      ui,
      w,
      target,
      0.0..=(f32::PI() / 4.0),
    );
    ui.label("spotlight cutoff distance:");
    modify_ranged_value_like_slider_com::<SpotLightCutOffDistance>(ui, w, target, 0.0..=10.);

    modify_bool_com::<BasicShadowMapEnabledOf<SpotLightBasicShadowInfo>>(
      ui,
      w,
      target,
      "shadow enabled",
    );
    //
  } else if let Some(target) = selection.selected_point_light {
    let w = &mut scene_writer.point_light_writer;
    ui.label(format!("Scene point light id: {:?}", target.into_raw()));

    show_entity_label(w, target, ui);
    modify_bool_com::<PointLightEnabled>(ui, w, target, "enabled");

    ui.label("spotlight cutoff distance:");
    modify_ranged_value_like_slider_com::<PointLightCutOffDistance>(ui, w, target, 0.0..=10.);
  } else {
    ui.label("No target selected");
  }

  Some(())
}
