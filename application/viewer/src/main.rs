#![feature(impl_trait_in_assoc_type)]
#![feature(stmt_expr_attributes)]
#![feature(type_alias_impl_trait)]
#![feature(hash_raw_entry)]
#![allow(clippy::collapsible_match)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]

use std::{alloc::System, sync::Arc};

use database::*;
use egui_winit::winit::{
  event::{Event, WindowEvent},
  event_loop::EventLoop,
  window::WindowBuilder,
};

mod egui_cx;
mod viewer;

use egui_cx::EguiContext;
use heap_tools::*;
use rendiation_scene_core::*;
use rendiation_texture_core::Size;
use rendiation_webgpu::*;
use viewer::*;

#[global_allocator]
static GLOBAL_ALLOCATOR: PreciseAllocationStatistics<System> =
  PreciseAllocationStatistics::new(System);

fn main() {
  env_logger::builder().init();

  setup_global_database(Default::default());
  register_scene_core_data_model();

  futures::executor::block_on(run())
}

#[allow(clippy::single_match)]
pub async fn run() {
  let event_loop = EventLoop::new().unwrap();
  let window = WindowBuilder::new().build(&event_loop).unwrap();
  window.set_title("viewer");

  let minimal_required_features = rendiation_webgpu::Features::all_webgpu_mask();
  // minimal_required_features.insert(Features::TEXTURE_BINDING_ARRAY);
  // minimal_required_features.insert(Features::BUFFER_BINDING_ARRAY);
  // minimal_required_features.insert(Features::PARTIALLY_BOUND_BINDING_ARRAY);

  let config = GPUCreateConfig {
    surface_for_compatible_check_init: Some((&window, Size::from_usize_pair_min_one((300, 200)))),
    minimal_required_features,
    ..Default::default()
  };

  let (gpu, surface) = GPU::new(config).await.unwrap();
  let gpu = Arc::new(gpu);

  let mut surface: GPUSurface<'static> = unsafe { std::mem::transmute(surface.unwrap()) };

  let mut viewer = Viewer::default();
  let mut window_state = WindowState::default();
  let mut egui_cx = EguiContext::new(&gpu.device, surface.config.format, None, 1, &window);

  let _ = event_loop.run(move |event, target| {
    window_state.event(&event);
    let position_info = CanvasWindowPositionInfo::full_window(window_state.size);
    viewer.event(&event, &window_state, position_info);

    match event {
      Event::WindowEvent { ref event, .. } => {
        egui_cx.handle_input(&window, event);

        match event {
          WindowEvent::CloseRequested => {
            target.exit();
          }
          WindowEvent::Resized(physical_size) => {
            // should we put this in viewer's event handler?
            viewer.update_render_size(window_state.size);
            surface.resize(
              Size::from_u32_pair_min_one((physical_size.width, physical_size.height)),
              &gpu.device,
            )
          }
          WindowEvent::RedrawRequested => {
            let (output, canvas) = surface.get_current_frame_with_render_target_view().unwrap();

            viewer.draw_canvas(&gpu, &canvas);

            egui_cx.begin_frame(&window);

            ui_logic(egui_cx.cx(), &mut viewer);

            egui_cx.end_frame_and_draw(&gpu, &window, &canvas);

            output.present();
            window.request_redraw();
          }

          _ => {}
        };
      }
      _ => {}
    }
  });
}

fn ui_logic(ui: &egui::Context, viewer: &mut Viewer) {
  egui::Window::new("Viewer")
    .vscroll(true)
    .default_open(true)
    .max_width(1000.0)
    .max_height(800.0)
    .default_width(800.0)
    .resizable(true)
    .movable(true)
    .anchor(egui::Align2::LEFT_TOP, [3.0, 3.0])
    .show(ui, |ui| {
      if ui.add(egui::Button::new("Click me")).clicked() {
        println!("PRESSED")
      }

      if let Some(ctx) = &mut viewer.ctx {
        ui_render_config(ui, &mut ctx.pipeline)
      }

      viewer
        .terminal
        .egui(ui, viewer.ctx.as_mut(), &viewer.io_executor);
    });
}

fn ui_render_config(ui: &mut egui::Ui, config: &mut ViewerPipeline) {
  ui.checkbox(&mut config.enable_ssao, "enable ssao");
  ui.checkbox(&mut config.enable_channel_debugger, "enable channel debug");
}
