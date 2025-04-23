use crate::*;

mod console;
mod db_view;

pub use console::*;
use db_view::*;

pub struct ViewerUIState {
  show_db_inspector: bool,
  show_viewer_config_panel: bool,
  show_terminal: bool,
  show_gpu_info: bool,
  show_memory_stat: bool,
  show_frame_info: bool,
  object_inspection: bool,
  egui_db_inspector: DBInspector,
}

impl Default for ViewerUIState {
  fn default() -> Self {
    Self {
      show_db_inspector: false,
      show_viewer_config_panel: true,
      show_terminal: false,
      show_frame_info: false,
      show_gpu_info: false,
      show_memory_stat: false,
      object_inspection: false,
      egui_db_inspector: Default::default(),
    }
  }
}

impl ViewerUIState {
  pub fn egui(
    &mut self,
    terminal: &mut Terminal,
    background: &mut ViewerBackgroundState,
    on_demand_rendering: &mut bool,
    rendering: &mut Viewer3dRenderingCtx,
    scene: EntityHandle<SceneEntity>,
    ui: &egui::Context,
    cx: &mut DynCx,
  ) {
    egui::TopBottomPanel::top("view top menu").show(ui, |ui| {
      ui.horizontal_wrapped(|ui| {
        egui::widgets::global_theme_preference_switch(ui);
        ui.separator();
        ui.checkbox(&mut self.show_db_inspector, "database inspector");
        ui.checkbox(&mut self.show_viewer_config_panel, "viewer config");
        ui.checkbox(&mut self.object_inspection, "object panel");
        ui.checkbox(&mut self.show_terminal, "terminal");
        ui.checkbox(&mut self.show_gpu_info, "gpu info");
        ui.checkbox(&mut self.show_frame_info, "frame info");
        ui.checkbox(&mut self.show_memory_stat, "heap stat");
      });
    });

    egui::Window::new("Memory")
      .vscroll(true)
      .open(&mut self.show_memory_stat)
      .show(ui, |ui| {
        #[cfg(feature = "heap-debug")]
        {
          if ui.button("reset heap peak stat").clicked() {
            GLOBAL_ALLOCATOR.reset_history_peak();
          }
          let stat = GLOBAL_ALLOCATOR.report();
          ui.label(format!("{:#?}", stat));
          if ui.button("reset counter peak stat").clicked() {
            heap_tools::HEAP_TOOL_GLOBAL_INSTANCE_COUNTER
              .write()
              .reset_all_instance_history_peak();
          }
          let global = heap_tools::HEAP_TOOL_GLOBAL_INSTANCE_COUNTER.read();
          for (ty, report) in global.report_all_instance_count() {
            ui.label(format!(
              "{ty} => current: {}, peak: {}",
              report.current, report.history_peak,
            ));
          }
        }
        #[cfg(not(feature = "heap-debug"))]
        {
          ui.label("heap stat is not enabled in this build");
        }
      });

    egui::Window::new("Viewer")
      .vscroll(true)
      .open(&mut self.show_viewer_config_panel)
      .default_pos([10., 60.])
      .max_width(1000.0)
      .max_height(800.0)
      .default_width(250.0)
      .default_height(400.0)
      .resizable(true)
      .movable(true)
      .show(ui, |ui| {
        ui.checkbox(on_demand_rendering, "enable on demand rendering");
        ui.separator();
        rendering.egui(ui);
        ui.separator();

        background.egui(ui, scene);

        ui.separator();

        ui.collapsing("Instance Counts", |ui| {
          let mut counters = heap_tools::HEAP_TOOL_GLOBAL_INSTANCE_COUNTER.write();

          for (name, r) in counters.report_all_instance_count() {
            ui.label(format!(
              "{}: current:{} peak:{}",
              get_short_name(name),
              r.current,
              r.history_peak
            ));
          }

          if ui.button("reset peak").clicked() {
            counters.reset_all_instance_history_peak();
          }
        });
      });

    egui::Window::new("Frame Rendering Info")
      .open(&mut self.show_frame_info)
      .vscroll(true)
      .show(ui, |ui| {
        ui.label("frame pass pipeline statistics:");
        ui.separator();

        ui.checkbox(
          &mut rendering.enable_statistic_collect,
          "enable_statistic_collect",
        );

        if rendering.enable_statistic_collect {
          if rendering.statistics.collected.is_empty() {
            ui.label("no statistics info available");
          } else {
            if !rendering.statistics.pipeline_query_supported {
              ui.label("note: pipeline query not supported on this platform");
            } else {
              let statistics = &mut rendering.statistics;
              ui.collapsing("pipeline_info", |ui| {
                statistics.collected.iter().for_each(|(name, info)| {
                  if let Some(info) = &info.pipeline.latest_resolved {
                    #[allow(dead_code)]
                    #[derive(Debug)] // just to impl Debug
                    struct DeviceDrawStatistics2 {
                      pub vertex_shader_invocations: u64,
                      pub clipper_invocations: u64,
                      pub clipper_primitives_out: u64,
                      pub fragment_shader_invocations: u64,
                      pub compute_shader_invocations: u64,
                    }

                    impl From<DeviceDrawStatistics> for DeviceDrawStatistics2 {
                      fn from(value: DeviceDrawStatistics) -> Self {
                        Self {
                          vertex_shader_invocations: value.vertex_shader_invocations,
                          clipper_invocations: value.clipper_invocations,
                          clipper_primitives_out: value.clipper_primitives_out,
                          fragment_shader_invocations: value.fragment_shader_invocations,
                          compute_shader_invocations: value.compute_shader_invocations,
                        }
                      }
                    }

                    ui.collapsing(name, |ui| {
                      ui.label(format!("frame index: {:?}", info.1));
                      ui.label(format!("{:#?}", DeviceDrawStatistics2::from(info.0)));
                    });
                  }
                });
              });
            }
            if !rendering.statistics.time_query_supported {
              ui.label("warning: time query not supported");
            } else {
              let statistics = &mut rendering.statistics;
              ui.collapsing("time_info", |ui| {
                statistics.collected.iter().for_each(|(name, info)| {
                  if let Some(info) = &info.time.latest_resolved {
                    let name = format!("{}: {:.2}ms", name, info.0);
                    ui.label(name);
                  }
                });
              });
            }

            if ui.button("clear").clicked() {
              rendering
                .statistics
                .clear_history(rendering.statistics.max_history);
            }
          }
        }
      });

    egui::Window::new("GPU Info")
      .open(&mut self.show_gpu_info)
      .vscroll(true)
      .show(ui, |ui| {
        let gpu = rendering.gpu();
        let info = &gpu.info;

        let mut enable_bind_check = gpu.device.get_binding_ty_check_enabled();
        ui.checkbox(&mut enable_bind_check, "enable bind type check");
        gpu.device.set_binding_ty_check_enabled(enable_bind_check);

        ui.separator();

        ui.collapsing("wgpu encapsulation layer info", |ui| {
          let cache_info = gpu.create_cache_report();

          ui.label(format!("{:#?}", cache_info));
          if ui.button("clear cache").clicked() {
            gpu.clear_resource_cache();
          }
        });

        ui.collapsing("wgpu internal info", |ui| {
          let storage_info = gpu.device.generate_allocator_report();
          if let Some(storage_info) = storage_info {
            ui.label(format!("{:#?}", storage_info));
          } else {
            ui.label("The current backend do not support producing this report");
          }

          // let counter_info = gpu.device.get_internal_counters();
          // ui.label(format!(
          //   "note: wgpu compile-feature is required or counters info are all zero"
          // ));
          // ui.label(format!("{:?}", counter_info.hal)); // todo wgpu not impl Debug for this
        });

        ui.collapsing("adaptor info", |ui| {
          ui.label(format!("{:#?}", info.adaptor_info));
          ui.label(format!("power preference: {:?}", info.power_preference));
        });

        ui.collapsing("supported_features", |ui| {
          for (supported_feature, _) in info.supported_features.iter_names() {
            ui.label(format!("{:?}", supported_feature));
          }
        });

        ui.collapsing("supported limits", |ui| {
          ui.label(format!("{:#?}", info.supported_limits));
        });
      });

    if self.show_terminal {
      egui::TopBottomPanel::bottom("view bottom terminal")
        .resizable(true)
        .show(ui, |ui| {
          cx.scoped_cx(rendering, |cx| {
            terminal.egui(ui, cx);
          });
        });
    }

    if self.object_inspection {
      egui::Window::new("Object Inspection")
        .open(&mut self.object_inspection)
        .vscroll(true)
        .show(ui, |_ui| {
          //
        });
    }

    egui_db_gui(ui, &mut self.egui_db_inspector, &mut self.show_db_inspector);
  }
}
