#![feature(impl_trait_in_assoc_type)]
#![feature(specialization)]
#![feature(stmt_expr_attributes)]
#![feature(type_alias_impl_trait)]
#![feature(hash_raw_entry)]
#![allow(clippy::collapsible_match)]

use std::{alloc::System, sync::Arc};

use egui_winit::winit::{
  event::{Event, WindowEvent},
  event_loop::EventLoop,
  window::WindowBuilder,
};
use rendiation_scene_core::*;

mod ui;
mod viewer;

use heap_tools::*;
use rendiation_scene_webgpu::*;
use ui::EguiRenderer;
pub use viewer::*;
use webgpu::{GPUCreateConfig, GPUSurface, GPU};

#[global_allocator]
static GLOBAL_ALLOCATOR: PreciseAllocationHook<System> = PreciseAllocationHook::new(System);

fn main() {
  setup_active_plane(Default::default());
  register_viewer_extra_scene_features();

  env_logger::builder().init();

  futures::executor::block_on(run())
}

#[allow(clippy::single_match)]
pub async fn run() {
  let event_loop = EventLoop::new().unwrap();
  let window = WindowBuilder::new().build(&event_loop).unwrap();
  window.set_title("viewer");

  let minimal_required_features = webgpu::Features::all_webgpu_mask();
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
  let mut egui_renderer = EguiRenderer::new(&gpu.device, surface.config.format, None, 1, &window);
  let mut ui_state = ViewerUIState::default();

  let _ = event_loop.run(move |event, target| {
    window_state.event(&event);
    let position_info = CanvasWindowPositionInfo::full_window(window_state.size);
    viewer.event(&event, &window_state, position_info);

    match event {
      Event::WindowEvent { ref event, .. } => {
        egui_renderer.handle_input(&window, event);

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
            let view = output.texture.create_view(&webgpu::TextureViewDescriptor {
              label: None,
              format: None,
              dimension: None,
              aspect: webgpu::TextureAspect::All,
              base_mip_level: 0,
              mip_level_count: None,
              base_array_layer: 0,
              array_layer_count: None,
            });

            viewer.draw_canvas(&gpu, canvas);

            let mut encoder =
              gpu
                .device
                .create_command_encoder(&webgpu::CommandEncoderDescriptor {
                  label: Some("Render Encoder"),
                });

            let screen_descriptor = egui_wgpu::ScreenDescriptor {
              size_in_pixels: [window.inner_size().width, window.inner_size().height],
              pixels_per_point: window.scale_factor() as f32,
            };

            egui_renderer.draw(
              &gpu.device,
              &gpu.queue,
              &mut encoder,
              &window,
              &view,
              screen_descriptor,
              |ctx| ui_logic(ctx, &mut ui_state, &mut viewer),
            );

            gpu.queue.submit(std::iter::once(encoder.finish()));
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

#[derive(Default)]
struct ViewerUIState {
  command_input: String,
}

fn ui_logic(ui: &egui::Context, states: &mut ViewerUIState, viewer: &mut Viewer) {
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

      ui.label("terminal");
      let re = ui.text_edit_singleline(&mut states.command_input);
      if re.lost_focus() && re.ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
        viewer.terminal_input.emit(&states.command_input);
        states.command_input = "".to_string();
      }
      ui.end_row();
    });
}

fn ui_render_config(ui: &mut egui::Ui, config: &mut ViewerPipeline) {
  ui.checkbox(&mut config.enable_ssao, "enable ssao");
  ui.checkbox(&mut config.enable_channel_debugger, "enable channel debug");
}
