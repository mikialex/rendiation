use crate::*;

mod console;
mod db_view;
mod inspector;
mod object_inspect;
mod tile;

pub use console::*;
use db_view::*;
pub use inspector::*;
pub use object_inspect::*;
pub use tile::*;

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

pub fn use_viewer_egui(cx: &mut ViewerCx) {
  let (cx, ui_state) = cx.use_plain_state::<ViewerUIState>();

  let (cx, console) = cx.use_plain_state::<Console>();

  let (cx, frame_cpu_time_stat) = cx.use_plain_state_init(|_| StatisticStore::<f32>::new(200));

  frame_cpu_time_stat.insert(
    cx.input.last_frame_cpu_time_in_ms,
    cx.viewer.rendering_root.frame_index(),
  );

  if let ViewerCxStage::Gui {
    egui_ui: ui,
    global,
    ..
  } = &mut cx.stage
  {
    let viewer = &mut cx.viewer;

    egui::Panel::top("view top menu").show_inside(ui, |ui| {
      ui.horizontal_wrapped(|ui| {
        egui::widgets::global_theme_preference_switch(ui);
        ui.separator();
        ui.checkbox(&mut ui_state.show_db_inspector, "database inspector");
        ui.checkbox(&mut ui_state.show_viewer_config_panel, "viewer config");
        ui.checkbox(&mut ui_state.object_inspection, "object panel");
        ui.checkbox(&mut ui_state.show_terminal, "terminal");
        ui.checkbox(&mut ui_state.show_gpu_info, "gpu info");
        ui.checkbox(&mut ui_state.show_frame_info, "frame info");
        ui.checkbox(&mut ui_state.show_memory_stat, "heap stat");
      });
    });

    egui::Window::new("Memory")
      .vscroll(true)
      .open(&mut ui_state.show_memory_stat)
      .show(ui, |ui| {
        #[cfg(feature = "heap-debug")]
        {
          ui.label(format!(
            "global living frame bumper: {}",
            get_global_living_bump()
          ));
          if ui.button("reset heap peak stat").clicked() {
            GLOBAL_ALLOCATOR.reset_history_peak();
          }
          let stat = GLOBAL_ALLOCATOR.report();
          GLOBAL_ALLOCATOR.reset_allocation_event_counter();
          ui.label(format!("{:#?}", stat));
          if ui.button("reset counter peak stat").clicked() {
            heap_tools::HEAP_TOOL_GLOBAL_INSTANCE_COUNTER
              .write()
              .reset_all_instance_history_peak();
          }
          let global = heap_tools::HEAP_TOOL_GLOBAL_INSTANCE_COUNTER.read();
          for (ty, report) in global.report_all_instance_count() {
            let ty = disqualified::ShortName(ty);
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
      .open(&mut ui_state.show_viewer_config_panel)
      .default_pos([10., 60.])
      .max_width(1000.0)
      .max_height(800.0)
      .default_width(250.0)
      .default_height(400.0)
      .resizable(true)
      .movable(true)
      .show(ui, |ui| {
        ui.collapsing("features", |ui| {
          for (name, show_panel) in global.features.iter_mut() {
            ui.checkbox(show_panel, *name);
          }
        });

        ui.separator();

        let is_hdr = cx.current_window_swapchain.is_hdr();
        let changed = viewer.rendering.egui(ui, is_hdr, cx.surface_id);

        if changed {
          viewer.rendering_root.notify_change();
        }

        cx.active_surface_content
          .background
          .egui(ui, cx.active_surface_content.scene);

        ui.separator();

        ui.checkbox(&mut viewer.enable_inspection, "enable_inspection");

        ui.checkbox(&mut viewer.use_scene_bvh, "use_scene_bvh");

        ui.collapsing("Init config(not dynamic configurable)", |ui| {
          if ui
            .button("export first viewport init and current config")
            .clicked()
          {
            let config = viewer.export_init_config(cx.current_window_swapchain);
            config.export_to_current_dir();
            cx.app_features.export_to_current_dir();
          }
          ui.label(format!("{:#?}", viewer.rendering.init_config().init_only));
        });
      });

    viewer.rendering_root.egui(
      ui,
      &mut ui_state.show_frame_info,
      cx.input.last_frame_cpu_time_in_ms,
      frame_cpu_time_stat,
      cx.current_window_swapchain,
    );

    egui::Window::new("GPU Info")
      .open(&mut ui_state.show_gpu_info)
      .vscroll(true)
      .show(ui, |ui| {
        let gpu = viewer.rendering.gpu();
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

    if ui_state.show_terminal {
      egui::Panel::bottom("view bottom terminal")
        .resizable(true)
        .show_inside(ui, |ui| {
          console.egui(ui, &mut viewer.terminal);
        });
    }
    viewer.terminal.tick_execute(
      &mut TerminalInitExecuteCx {
        scene: &cx.active_surface_content,
        renderer: &mut viewer.rendering,
        dyn_cx: cx.dyn_cx,
      },
      &mut |output| {
        console.writeln(output);
      },
    );

    if ui_state.object_inspection {
      egui::Window::new("Object Inspection")
        .open(&mut ui_state.object_inspection)
        .vscroll(true)
        .show(ui, |ui| {
          inspect_selected(
            ui,
            &mut cx.viewer.selection,
            cx.active_surface_content.scene,
          );
          Some(())
        });
    }

    egui_db_gui(
      ui,
      &mut ui_state.egui_db_inspector,
      &mut ui_state.show_db_inspector,
    );
  }
}

pub fn modify_color4(ui: &mut egui::Ui, c: &mut Vec4<f32>) {
  let mut color: [f32; 4] = (*c).into();
  ui.color_edit_button_rgba_unmultiplied(&mut color);
  *c = color.into();
}

pub fn modify_color(ui: &mut egui::Ui, c: &mut Vec3<f32>) {
  let mut color: [f32; 3] = (*c).into();
  ui.color_edit_button_rgb(&mut color);
  *c = color.into();
}

pub fn modify_color_like_com<C: ComponentSemantic<Data = Vec3<f32>>>(
  ui: &mut egui::Ui,
  writer: &mut EntityWriter<C::Entity>,
  id: EntityHandle<C::Entity>,
) {
  let mut color = writer.read::<C>(id);
  modify_color(ui, &mut color);
  writer.write::<C>(id, color);
}

pub fn modify_normalized_value_like_com<C: ComponentSemantic<Data = f32>>(
  ui: &mut egui::Ui,
  writer: &mut EntityWriter<C::Entity>,
  id: EntityHandle<C::Entity>,
) {
  modify_ranged_value_like_slider_com::<C>(ui, writer, id, 0.0..=1.0);
}

pub fn modify_ranged_value_like_slider_com<C: ComponentSemantic<Data = f32>>(
  ui: &mut egui::Ui,
  writer: &mut EntityWriter<C::Entity>,
  id: EntityHandle<C::Entity>,
  range: std::ops::RangeInclusive<f32>,
) {
  let mut v = writer.read::<C>(id);

  ui.add(egui::Slider::new(&mut v, range).step_by(0.05));

  writer.write::<C>(id, v);
}

pub fn modify_bool_com<C: ComponentSemantic<Data = bool>>(
  ui: &mut egui::Ui,
  writer: &mut EntityWriter<C::Entity>,
  id: EntityHandle<C::Entity>,
  label: &str,
) {
  let mut v = writer.read::<C>(id);

  ui.checkbox(&mut v, label);

  writer.write::<C>(id, v);
}

pub fn show_entity_label<E: EntitySemantic>(
  writer: &EntityWriter<E>,
  target: EntityHandle<E>,
  ui: &mut egui::Ui,
) {
  let label = writer.read::<LabelOf<E>>(target);
  if !label.is_empty() {
    ui.label(format!("label: {}", label));
  }
}
