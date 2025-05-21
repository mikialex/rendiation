use crate::*;

impl Viewer3dRenderingCtx {
  pub fn egui(&mut self, ui: &mut egui::Ui) {
    let mut is_hdr = false;
    self.swap_chain.internal(|surface| {
      is_hdr = surface.config.format == TextureFormat::Rgba16Float;
      ui.collapsing("Swapchain config", |ui| {
        let cap = surface.capabilities();
        let default_none_hdr_format = get_default_preferred_format(cap);
        let support_hdr = cap.formats.contains(&TextureFormat::Rgba16Float);

        ui.add_enabled_ui(support_hdr, |ui| {
          ui.checkbox(&mut is_hdr, "enable hdr rendering")
            .on_disabled_hover_text("current platform does not support hdr rendering");
          if is_hdr {
            surface.config.format = TextureFormat::Rgba16Float;
          } else {
            surface.config.format = default_none_hdr_format;
          }
        });

        egui::ComboBox::from_label("present mode")
          .selected_text(format!("{:?}", &surface.config.present_mode))
          .show_ui(ui, |ui| {
            ui.selectable_value(
              &mut surface.config.present_mode,
              PresentMode::AutoVsync,
              "AutoVsync",
            );
            ui.selectable_value(
              &mut surface.config.present_mode,
              PresentMode::AutoNoVsync,
              "AutoNoVsync",
            );
            ui.selectable_value(&mut surface.config.present_mode, PresentMode::Fifo, "Fifo");
            ui.selectable_value(
              &mut surface.config.present_mode,
              PresentMode::FifoRelaxed,
              "FifoRelaxed",
            );
            ui.selectable_value(
              &mut surface.config.present_mode,
              PresentMode::Immediate,
              "Immediate",
            );
            ui.selectable_value(
              &mut surface.config.present_mode,
              PresentMode::Mailbox,
              "Mailbox",
            );
          });
      });
    });

    let is_target_support_indirect_draw = self
      .gpu
      .info
      .supported_features
      .contains(Features::MULTI_DRAW_INDIRECT_COUNT);

    let old = self.current_renderer_impl_ty;
    egui::ComboBox::from_label("RasterizationRender Backend")
      .selected_text(format!("{:?}", &self.current_renderer_impl_ty))
      .show_ui(ui, |ui| {
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
          .on_disabled_hover_text("current platform/gpu does not support indirect rendering");
        });
      });

    ui.separator();

    egui::ComboBox::from_label("Lighting technique for opaque objects")
      .selected_text(format!(
        "{:?}",
        &self.lighting.opaque_scene_content_lighting_technique
      ))
      .show_ui(ui, |ui| {
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

    if old != self.current_renderer_impl_ty {
      self.renderer_impl.deregister(&mut self.rendering_resource);
      self.renderer_impl = init_renderer(
        &mut self.rendering_resource,
        self.current_renderer_impl_ty,
        &self.gpu,
        self.camera_source.clone_as_static(),
        self.ndc.enable_reverse_z,
      );
    }

    ui.add_enabled_ui(is_target_support_indirect_draw, |ui| {
      let mut indirect_occlusion_culling_impl_exist =
        self.indirect_occlusion_culling_impl.is_some();
      ui.checkbox(
        &mut indirect_occlusion_culling_impl_exist,
        "occlusion_culling_system_is_ready",
      )
      .on_disabled_hover_text("current platform/gpu does not support gpu driven occlusion culling");
      self.set_enable_indirect_occlusion_culling_support(indirect_occlusion_culling_impl_exist);
    });

    ui.add_enabled_ui(true, |ui| {
      let mut rtx_renderer_impl_exist = self.rtx_renderer_impl.is_some();
      ui.checkbox(&mut rtx_renderer_impl_exist, "rtx_renderer_is_ready");
      self.set_enable_rtx_rendering_support(rtx_renderer_impl_exist);

      if let Some(renderer) = &self.rtx_renderer_impl {
        ui.checkbox(&mut self.rtx_rendering_enabled, "enable ray tracing");
        egui::ComboBox::from_label("ray tracing mode")
          .selected_text(format!("{:?}", &self.rtx_effect_mode))
          .show_ui(ui, |ui| {
            ui.selectable_value(
              &mut self.rtx_effect_mode,
              RayTracingEffectMode::ReferenceTracing,
              "Path tracing",
            );
            ui.selectable_value(
              &mut self.rtx_effect_mode,
              RayTracingEffectMode::AO,
              "AO only",
            );
          });

        match self.rtx_effect_mode {
          RayTracingEffectMode::AO => {
            if ui.button("reset ao sample").clicked() {
              renderer.ao.reset_ao_sample();
            }
          }
          RayTracingEffectMode::ReferenceTracing => {
            if ui.button("reset pt sample").clicked() {
              renderer.pt.reset_sample();
            }
          }
        }
      }
    });

    ui.separator();

    self.lighting.egui(ui, is_hdr);
    self.frame_logic.egui(ui);
  }
}
