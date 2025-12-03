use crate::*;

mod change;
mod console;
mod db_view;
mod inspector;
mod tile;

pub use change::*;
pub use console::*;
use db_view::*;
pub use inspector::*;
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

  let (cx, frame_cpu_time_stat) = cx.use_plain_state_init(|_| StatisticStore::<f32>::new(200));

  frame_cpu_time_stat.insert(
    cx.input.last_frame_cpu_time_in_ms,
    cx.viewer.rendering_root.frame_index(),
  );

  if let ViewerCxStage::Gui {
    egui_ctx: ui,
    global,
    ..
  } = &mut cx.stage
  {
    let viewer = &mut cx.viewer;

    egui::TopBottomPanel::top("view top menu").show(ui, |ui| {
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

        let is_hdr = viewer.rendering_root.is_hdr();
        let changed = viewer.rendering.egui(ui, is_hdr);

        if changed {
          viewer.rendering_root.notify_change();
        }

        viewer.background.egui(ui, viewer.content.scene);

        ui.separator();

        ui.checkbox(&mut viewer.enable_inspection, "enable_inspection");

        ui.collapsing("Init config(not dynamic configurable)", |ui| {
          if ui
            .button("export first viewport init and current config")
            .clicked()
          {
            let config = viewer.export_init_config();
            config.export_to_current_dir();
          }
          ui.label(format!("{:#?}", viewer.rendering.init_config.init_only));
        });

        ui.collapsing("Rendering Resources Detail", |ui| {
          struct EguiInspector<'a>(&'a mut egui::Ui);
          impl<'a> Inspector for EguiInspector<'a> {
            fn label(&mut self, label: &str) {
              self.0.label(label);
            }
          }
          let mut inspector = EguiInspector(ui);

          viewer.rendering_root.inspect(
            &mut viewer.shared_ctx,
            &mut inspector as &mut dyn Inspector,
            &mut viewer.rendering,
            &viewer.content.viewports,
          );
        });
      });

    viewer.rendering_root.egui(
      ui,
      &mut ui_state.show_frame_info,
      cx.input.last_frame_cpu_time_in_ms,
      frame_cpu_time_stat,
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
      egui::TopBottomPanel::bottom("view bottom terminal")
        .resizable(true)
        .show(ui, |ui| {
          viewer.terminal.egui(ui);
        });
    }
    viewer.terminal.tick_execute(&mut TerminalInitExecuteCx {
      scene: &viewer.content,
      renderer: &mut viewer.rendering,
      dyn_cx: cx.dyn_cx,
    });

    if ui_state.object_inspection {
      egui::Window::new("Object Inspection")
        .open(&mut ui_state.object_inspection)
        .vscroll(true)
        .show(ui, |ui| {
          let mut scene_writer = SceneWriter::from_global(viewer.content.scene);
          if let Some(target) = viewer.content.selected_model {
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

            let std_model = scene_writer
              .model_writer
              .read_foreign_key::<SceneModelStdModelRenderPayload>(target)?;

            ui.label(format!(
              "referenced std_model id: {:?}",
              std_model.into_raw()
            ));
            show_entity_label(&scene_writer.std_model_writer, std_model, ui);

            ui.separator();

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
          } else if let Some(target) = viewer.content.selected_spot_light {
            ui.label(format!("Scene Spotlight id: {:?}", target.into_raw()));
            show_entity_label(&scene_writer.spot_light_writer, target, ui);
            ui.label("spotlight half cone angle:");
            modify_ranged_value_like_slider_com::<SpotLightHalfConeAngle>(
              ui,
              &mut scene_writer.spot_light_writer,
              target,
              0.0..=(f32::PI() / 4.0),
            );
            ui.label("spotlight penumbra angle:");
            modify_ranged_value_like_slider_com::<SpotLightHalfPenumbraAngle>(
              ui,
              &mut scene_writer.spot_light_writer,
              target,
              0.0..=(f32::PI() / 4.0),
            );
            ui.label("spotlight cutoff distance:");
            modify_ranged_value_like_slider_com::<SpotLightCutOffDistance>(
              ui,
              &mut scene_writer.spot_light_writer,
              target,
              0.0..=10.,
            );
          } else if let Some(target) = viewer.content.selected_point_light {
            ui.label(format!("Scene point light id: {:?}", target.into_raw()));
            show_entity_label(&scene_writer.point_light_writer, target, ui);
            ui.label("spotlight cutoff distance:");
            modify_ranged_value_like_slider_com::<PointLightCutOffDistance>(
              ui,
              &mut scene_writer.point_light_writer,
              target,
              0.0..=10.,
            );
          } else {
            ui.label("No target selected");
          }

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

fn modify_color_like_com<C: ComponentSemantic<Data = Vec3<f32>>>(
  ui: &mut egui::Ui,
  writer: &mut EntityWriter<C::Entity>,
  id: EntityHandle<C::Entity>,
) {
  let mut color = writer.read::<C>(id);
  modify_color(ui, &mut color);
  writer.write::<C>(id, color);
}

fn modify_normalized_value_like_com<C: ComponentSemantic<Data = f32>>(
  ui: &mut egui::Ui,
  writer: &mut EntityWriter<C::Entity>,
  id: EntityHandle<C::Entity>,
) {
  modify_ranged_value_like_slider_com::<C>(ui, writer, id, 0.0..=1.0);
}

fn modify_ranged_value_like_slider_com<C: ComponentSemantic<Data = f32>>(
  ui: &mut egui::Ui,
  writer: &mut EntityWriter<C::Entity>,
  id: EntityHandle<C::Entity>,
  range: std::ops::RangeInclusive<f32>,
) {
  let mut v = writer.read::<C>(id);

  ui.add(egui::Slider::new(&mut v, range).step_by(0.05));

  writer.write::<C>(id, v);
}

fn show_entity_label<E: EntitySemantic>(
  writer: &EntityWriter<E>,
  target: EntityHandle<E>,
  ui: &mut egui::Ui,
) {
  let label = writer.read::<LabelOf<E>>(target);
  if !label.is_empty() {
    ui.label(format!("label: {}", label));
  }
}
