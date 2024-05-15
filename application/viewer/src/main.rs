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

mod app;
mod egui_cx;
mod viewer;

use app::*;
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

  let viewer = StateCxCreateOnce::new(|cx| {
    state_access!(cx, gpu, Arc<GPU>);
    Viewer::new(gpu.clone())
  });
  let egui_view = EguiContext::new(viewer);

  let app_loop = run_application(egui_view);

  futures::executor::block_on(app_loop)
}
