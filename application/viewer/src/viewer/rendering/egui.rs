use crate::*;

impl Viewer3dRenderingCtx {
  #[must_use]
  pub fn egui(&mut self, ui: &mut egui::Ui, is_hdr: bool) -> bool {
    let mut ui = UiWithChangeInfo(ui, false);
    let ui = &mut ui;

    let is_target_support_indirect_draw = self.gpu.info.downgrade_info.is_webgpu_compliant()
      || (self
        .init_config
        .init_only
        .using_texture_as_storage_buffer_for_indirect_rendering
        && self.init_config.init_only.enable_indirect_storage_combine
        && self.using_host_driven_indirect_draw);

    egui::ComboBox::from_label("RasterizationRender Backend")
      .selected_text(format!("{:?}", &self.current_renderer_impl_ty))
      .show_ui_changed(ui, |ui| {
        ui.selectable_value(
          &mut self.current_renderer_impl_ty,
          RasterizationRenderBackendType::Gles,
          "Gles",
        );

        ui.add_enabled_ui(is_target_support_indirect_draw, |ui| {
          ui.selectable_value(
            &mut self.current_renderer_impl_ty,
            RasterizationRenderBackendType::Indirect,
            "Indirect",
          )
          .on_disabled_hover_text(
            "current platform/gpu or config combination does not support indirect rendering",
          );
        });
      });

    ui.checkbox(
      &mut self.prefer_bindless_for_indirect_texture_system,
      "prefer_bindless_for_indirect_texture_system(when available)",
    );

    ui.checkbox(
      &mut self.using_host_driven_indirect_draw,
      "using_host_driven_indirect_draw",
    );

    if self.current_renderer_impl_ty == RasterizationRenderBackendType::Indirect {
      let is_target_support_indirect_draw_cmd_natively = self
        .gpu
        .info
        .supported_features
        .contains(Features::MULTI_DRAW_INDIRECT_COUNT);

      if !is_target_support_indirect_draw_cmd_natively {
        ui.label("warning: current platform's indirect draw will using downgraded implementation");
      }
    }

    ui.separator();

    egui::ComboBox::from_label("how to lighting opaque objects?")
      .selected_text(format!(
        "{:?}",
        &self.lighting.opaque_scene_content_lighting_technique
      ))
      .show_ui_changed(ui, |ui| {
        ui.selectable_value(
          &mut self.lighting.opaque_scene_content_lighting_technique,
          LightingTechniqueKind::Forward,
          "Forward",
        );

        ui.selectable_value(
          &mut self.lighting.opaque_scene_content_lighting_technique,
          LightingTechniqueKind::DeferLighting,
          "DeferLighting",
        )
      });

    ui.separator();

    let message = if !is_target_support_indirect_draw {
      "current platform/gpu does not support gpu driven occlusion culling"
    } else if is_target_support_indirect_draw
      && self.current_renderer_impl_ty != RasterizationRenderBackendType::Indirect
    {
      self.enable_indirect_occlusion_culling = false;
      "gpu driven occlusion culling only available when gpu driven rendering is enabled"
    } else {
      ""
    };

    ui.add_enabled_ui(
      self.current_renderer_impl_ty == RasterizationRenderBackendType::Indirect,
      |ui| {
        ui.checkbox(
          &mut self.enable_indirect_occlusion_culling,
          "enable_indirect_occlusion_culling",
        )
        .on_disabled_hover_text(message);
      },
    );

    ui.checkbox(&mut self.enable_frustum_culling, "enable_frustum_culling");

    ui.separator();

    self.lighting.egui(ui, is_hdr);

    ui.separator();

    ui.add_enabled_ui(true, |ui| {
      ui.checkbox(&mut self.rtx_renderer_enabled, "rtx_renderer_is_ready");
    });

    for (id, view) in self.views.iter_mut() {
      ui.collapsing(format!("view config {}", id), |ui| {
        view.egui(ui, self.rtx_renderer_enabled);
      });
    }

    ui.1
  }
}
