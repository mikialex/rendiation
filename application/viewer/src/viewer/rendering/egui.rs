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

    let is_target_support_indirect_draw = true; // todo, check pure gles platform

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

    egui::ComboBox::from_label("how to render transparent objects?")
      .selected_text(format!("{:?}", &self.transparent_config,))
      .show_ui(ui, |ui| {
        ui.selectable_value(
          &mut self.transparent_config,
          ViewerTransparentContentRenderStyle::NaiveAlphaBlend,
          "naive alpha blend",
        );

        ui.selectable_value(
          &mut self.transparent_config,
          ViewerTransparentContentRenderStyle::WeightedOIT,
          "oit weighted style",
        );

        ui.selectable_value(
          &mut self.transparent_config,
          ViewerTransparentContentRenderStyle::Loop32OIT,
          "oit loop32 style",
        )
      });

    ui.separator();

    ui.add_enabled_ui(is_target_support_indirect_draw, |ui| {
      ui.checkbox(
        &mut self.enable_indirect_occlusion_culling_support,
        "occlusion_culling_system_is_ready",
      )
      .on_disabled_hover_text("current platform/gpu does not support gpu driven occlusion culling");
    });

    ui.add_enabled_ui(true, |ui| {
      ui.checkbox(&mut self.rtx_renderer_enabled, "rtx_renderer_is_ready");

      if self.rtx_renderer_enabled {
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

        if ui.button("reset  sample").clicked() {
          self.request_reset_rtx_sample = true;
        }
      }
    });

    ui.separator();

    ui.collapsing("time graph", |ui| {
      if let Some((t, _)) = self.stat_frame_time_in_ms.get_latest() {
        ui.label(format!(
          "last frame time: {:.2} ms, fps: {:.2}",
          t,
          1000. / t
        ));
      }
      if let Some(times) = self.stat_frame_time_in_ms.iter_history_from_oldest_latest() {
        let graph_height = 200.;
        let graph_width = 300.;
        let (res, painter) = ui.allocate_painter(
          egui::Vec2 {
            x: graph_width,
            y: graph_height,
          },
          egui::Sense::empty(),
        );
        let x_start = res.rect.left();
        let y_start = res.rect.top();
        let x_step = graph_width / self.stat_frame_time_in_ms.history_size() as f32;

        let warning_time_threshold = 1000. / 60.;
        let serious_warning_time_threshold = 1000. / 15.;
        let max_time = self
          .stat_frame_time_in_ms
          .history_max()
          .copied()
          .unwrap_or(warning_time_threshold);
        for (idx, t) in times.enumerate() {
          if let Some(&t) = t {
            let height = t / max_time * graph_height;
            let color = if t >= serious_warning_time_threshold {
              egui::Color32::RED
            } else if t >= warning_time_threshold {
              egui::Color32::ORANGE
            } else if ui.visuals().dark_mode {
              egui::Color32::WHITE
            } else {
              egui::Color32::BLACK
            };
            painter.rect_filled(
              egui::Rect {
                min: egui::pos2(
                  x_start + idx as f32 * x_step,
                  y_start + (graph_height - height),
                ),
                max: egui::pos2(x_start + (idx + 1) as f32 * x_step, y_start + graph_height),
              },
              0.,
              color,
            );
          }
        }
      } else {
        ui.label("frame time graph not available");
      }
    });

    self.lighting.egui(ui, is_hdr);
    self.frame_logic.egui(ui);
  }
}
