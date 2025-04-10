#![feature(impl_trait_in_assoc_type)]
#![feature(array_chunks)]
#![feature(stmt_expr_attributes)]
#![feature(type_alias_impl_trait)]
#![feature(hash_raw_entry)]
#![feature(ptr_metadata)]
#![allow(clippy::collapsible_match)]

use std::alloc::System;
use std::any::Any;
use std::hash::Hash;

use database::*;
use futures::FutureExt;
use reactive::*;
use rendiation_area_lighting::register_area_lighting_data_model;
use rendiation_geometry::*;
use rendiation_gui_3d::*;
use rendiation_lighting_gpu_system::*;
use rendiation_lighting_transport::*;
use rendiation_scene_rendering_gpu_gles::*;
use rendiation_shader_api::*;
use rendiation_texture_gpu_base::SamplerConvertExt;
use tracing::*;
use winit::{
  event::{Event, WindowEvent},
  event_loop::EventLoop,
  window::Window,
};

mod app_loop;
mod egui_cx;
mod util;
mod viewer;

use app_loop::*;
use egui_cx::EguiContext;
use heap_tools::*;
use rendiation_scene_core::*;
use rendiation_texture_core::*;
use rendiation_webgpu::*;
use util::*;
pub use viewer::*;

#[cfg(feature = "tracy-heap-debug")]
#[global_allocator]
static GLOBAL_ALLOCATOR: PreciseAllocationStatistics<
  tracing_tracy::client::ProfiledAllocator<System>,
> = PreciseAllocationStatistics::new(tracing_tracy::client::ProfiledAllocator::new(System, 64));

#[cfg(not(feature = "tracy-heap-debug"))]
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
  register_gui3d_extension_data_model();
  register_area_lighting_data_model();
  register_sky_env_data_model();

  let content_logic = core_viewer_features(content_logic);

  let viewer = StateCxCreateOnce::create_at_view(|cx| {
    access_cx!(cx, gpu_and_surface, WGPUAndSurface);
    Viewer::new(
      gpu_and_surface.gpu.clone(),
      gpu_and_surface.surface.clone(),
      content_logic(cx),
    )
  });
  let egui_view = EguiContext::new(viewer);

  run_application(egui_view);
}

fn main() {
  #[cfg(feature = "tracy")]
  {
    use tracing_subscriber::prelude::*;
    tracing::subscriber::set_global_default(
      tracing_subscriber::registry().with(tracing_tracy::TracyLayer::default()),
    )
    .expect("setting tracing default failed");
  }

  run_viewer_app(|_| {});
}
