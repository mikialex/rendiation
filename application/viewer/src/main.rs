#![feature(impl_trait_in_assoc_type)]
#![feature(stmt_expr_attributes)]
#![feature(type_alias_impl_trait)]
#![feature(hash_raw_entry)]
#![allow(clippy::collapsible_match)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_variables)]

use std::any::Any;
use std::hash::Hash;
use std::{alloc::System, sync::Arc};

use database::*;
use egui_winit::winit::{
  event::{Event, WindowEvent},
  event_loop::EventLoop,
  window::WindowBuilder,
};
use reactive::*;
use rendiation_gizmo::*;
use rendiation_gui_3d::*;
use rendiation_lighting_transport::*;
use rendiation_scene_rendering_gpu_gles::*;
use rendiation_shader_api::*;

mod egui_cx;
mod viewer;

use egui_cx::EguiContext;
use heap_tools::*;
use rendiation_scene_core::*;
use rendiation_texture_core::*;
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

  let mut viewer = Viewer::new(gpu.clone());
  let mut window_state = WindowState::default();
  let mut egui_cx = EguiContext::new(&gpu.device, surface.config.format, None, 1, &window);

  let _ = event_loop.run(move |event, target| {
    window_state.event(&event);
    let position_info = CanvasWindowPositionInfo::full_window(window_state.size);

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

            let mut cx = StateCx::default();

            egui_cx.begin_frame(&window);

            // todo, cx register egui
            viewer.update_view(&mut cx);

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
