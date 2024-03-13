#![feature(impl_trait_in_assoc_type)]
#![feature(specialization)]
#![feature(stmt_expr_attributes)]
#![feature(type_alias_impl_trait)]
#![feature(hash_raw_entry)]
#![allow(clippy::collapsible_match)]

use std::alloc::System;

use egui_winit::winit::{
  event::{Event, WindowEvent},
  event_loop::EventLoop,
  window::WindowBuilder,
};
use rendiation_scene_core::*;
use rendiation_scene_webgpu::*;

mod ui;
// mod viewer;

use heap_tools::*;
use ui::EguiRenderer;
// pub use viewer::*;
use webgpu::{GPUCreateConfig, GPUSurface, GPU};

#[global_allocator]
static GLOBAL_ALLOCATOR: PreciseAllocationHook<System> = PreciseAllocationHook::new(System);

fn main() {
  setup_active_plane(Default::default());
  // register_viewer_extra_scene_features();

  env_logger::builder().init();

  futures::executor::block_on(run())
}

pub async fn run() {
  let event_loop = EventLoop::new().unwrap();
  let window = WindowBuilder::new().build(&event_loop).unwrap();

  let mut minimal_required_features = webgpu::Features::all_webgpu_mask();
  // minimal_required_features.insert(Features::TEXTURE_BINDING_ARRAY);
  // minimal_required_features.insert(Features::BUFFER_BINDING_ARRAY);
  // minimal_required_features.insert(Features::PARTIALLY_BOUND_BINDING_ARRAY);
  minimal_required_features.remove(webgpu::Features::TIMESTAMP_QUERY); // note: on macos we currently do not have this

  let config = GPUCreateConfig {
    surface_for_compatible_check_init: Some((&window, Size::from_usize_pair_min_one((300, 200)))),
    minimal_required_features,
    ..Default::default()
  };

  let (gpu, surface) = GPU::new(config).await.unwrap();

  let mut surface: GPUSurface<'static> = unsafe { std::mem::transmute(surface.unwrap()) };

  // let viewer = Viewer::new();
  let mut egui_renderer = EguiRenderer::new(&gpu.device, surface.config.format, None, 1, &window);

  let _ = event_loop.run(move |event, ewlt| match event {
    Event::WindowEvent { ref event, .. } => {
      // viewer.event(event, states, position_info);
      window.request_redraw();
      egui_renderer.handle_input(&window, event);

      match event {
        WindowEvent::Resized(physical_size) => {
          // viewer.update_render_size(physical_size);
          surface.resize(
            Size::from_u32_pair_min_one((physical_size.width, physical_size.height)),
            &gpu.device,
          )
        }
        WindowEvent::RedrawRequested => {
          // viewer.draw_canvas(gpu, canvas);

          let output = surface.get_current_frame().unwrap();
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

          let mut encoder = gpu
            .device
            .create_command_encoder(&webgpu::CommandEncoderDescriptor {
              label: Some("Render Encoder"),
            });

          let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [300, 200],
            pixels_per_point: window.scale_factor() as f32,
          };

          egui_renderer.draw(
            &gpu.device,
            &gpu.queue,
            &mut encoder,
            &window,
            &view,
            screen_descriptor,
            |ui| ui_logic(ui),
          );

          gpu.queue.submit(std::iter::once(encoder.finish()));
          output.present();
        }

        _ => {}
      };
    }
    _ => {}
  });
}

pub fn ui_logic(ui: &egui::Context) {
  egui::Window::new("Test Viewer")
    // .vscroll(true)
    .default_open(true)
    .max_width(1000.0)
    .max_height(800.0)
    .default_width(800.0)
    .resizable(true)
    .anchor(egui::Align2::LEFT_TOP, [0.0, 0.0])
    .show(&ui, |mut ui| {
      if ui.add(egui::Button::new("Click me")).clicked() {
        println!("PRESSED")
      }

      ui.label("Slider");
      // ui.add(egui::Slider::new(_, 0..=120).text("age"));
      ui.end_row();

      // proto_scene.egui(ui);
    });
}
