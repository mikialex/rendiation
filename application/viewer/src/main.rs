#![feature(impl_trait_in_assoc_type)]
#![feature(stmt_expr_attributes)]
#![feature(type_alias_impl_trait)]
#![feature(hash_raw_entry)]
#![allow(clippy::collapsible_match)]

use std::any::Any;
use std::hash::Hash;
use std::{alloc::System, sync::Arc};

use database::*;
use reactive::*;
use rendiation_geometry::*;
use rendiation_gizmo::*;
use rendiation_gui_3d::*;
use rendiation_lighting_transport::*;
use rendiation_scene_rendering_gpu_gles::*;
use rendiation_shader_api::*;
use winit::{
  event::{Event, WindowEvent},
  event_loop::EventLoop,
  window::WindowBuilder,
};

mod app_loop;
mod default_scene;
mod egui_cx;
mod viewer;
//  use default_scene::*;

use app_loop::*;
use egui_cx::EguiContext;
use heap_tools::*;
use rendiation_scene_core::*;
use rendiation_texture_core::*;
use rendiation_webgpu::*;
use viewer::*;

#[global_allocator]
static GLOBAL_ALLOCATOR: PreciseAllocationStatistics<System> =
  PreciseAllocationStatistics::new(System);

pub fn run_viewer_app<V>(content_logic: impl Fn(&mut DynCx) -> V + 'static)
where
  V: Widget + 'static,
{
  env_logger::builder().init();

  setup_global_database(Default::default());
  register_scene_core_data_model();

  let content_logic = core_viewer_features(content_logic);

  let viewer = StateCxCreateOnce::new(|cx| {
    access_cx!(cx, gpu, Arc<GPU>);
    Viewer::new(gpu.clone(), content_logic(cx))
  });
  let egui_view = EguiContext::new(viewer);

  let app_loop = run_application(egui_view);

  futures::executor::block_on(app_loop)
}

fn main() {
  run_viewer_app(|_| {});
}
