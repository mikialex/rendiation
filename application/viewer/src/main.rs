#![feature(impl_trait_in_assoc_type)]
#![feature(stmt_expr_attributes)]
#![feature(type_alias_impl_trait)]
#![feature(hash_raw_entry)]
#![feature(ptr_metadata)]
#![allow(clippy::collapsible_match)]

use std::alloc::System;
use std::any::Any;
use std::hash::Hash;

use database::*;
use reactive::*;
use rendiation_geometry::*;
// use rendiation_gizmo::*;
use rendiation_gui_3d::*;
use rendiation_lighting_gpu_system::*;
use rendiation_lighting_transport::*;
use rendiation_scene_rendering_gpu_gles::*;
use rendiation_shader_api::*;
use tracing::*;
use winit::{
  event::{Event, WindowEvent},
  event_loop::EventLoop,
  window::Window,
};

mod app_loop;
mod db_egui_view;
mod default_scene;
mod egui_cx;
mod util;
mod viewer;
//  use default_scene::*;

use app_loop::*;
use egui_cx::EguiContext;
use heap_tools::*;
use rendiation_scene_core::*;
use rendiation_texture_core::*;
use rendiation_webgpu::*;
use util::*;
pub use viewer::*;

#[global_allocator]
static GLOBAL_ALLOCATOR: PreciseAllocationStatistics<System> =
  PreciseAllocationStatistics::new(System);

pub fn run_viewer_app<V>(content_logic: impl Fn(&mut DynCx) -> V + 'static)
where
  V: Widget + 'static,
{
  env_logger::builder().init();

  setup_global_database(Default::default());
  setup_active_reactive_query_registry(Default::default());

  let watch = DatabaseMutationWatch::new(&global_database());
  let rev_watch = DatabaseEntityReverseReference::new(watch.clone());
  register_global_database_feature(watch);
  register_global_database_feature(rev_watch);

  register_scene_core_data_model();
  register_light_shadow_config();

  let content_logic = core_viewer_features(content_logic);

  let viewer = StateCxCreateOnce::new(|cx| {
    access_cx!(cx, gpu, GPU);
    Viewer::new(gpu.clone(), content_logic(cx))
  });
  let egui_view = EguiContext::new(viewer);

  let app_loop = run_application(egui_view);

  futures::executor::block_on(app_loop)
}

fn main() {
  use tracing_subscriber::prelude::*;
  tracing::subscriber::set_global_default(
    tracing_subscriber::registry().with(tracing_tracy::TracyLayer::default()),
  )
  .expect("setting tracing default failed");

  run_viewer_app(|_| {});
}
