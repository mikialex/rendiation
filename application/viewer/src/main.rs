#![feature(impl_trait_in_assoc_type)]
#![feature(file_buffered)]
#![feature(stmt_expr_attributes)]
#![feature(type_alias_impl_trait)]
#![feature(ptr_metadata)]
#![allow(clippy::collapsible_match)]

use std::alloc::System;
use std::any::Any;
use std::future::Future;
use std::hash::Hash;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

use database::*;
use event_source::*;
use futures::FutureExt;
use futures::StreamExt;
use parking_lot::RwLock;
use rendiation_area_lighting::register_area_lighting_data_model;
use rendiation_geometry::*;
use rendiation_gui_3d::*;
use rendiation_lighting_gpu_system::*;
use rendiation_lighting_transport::*;
use rendiation_mesh_core::*;
use rendiation_mesh_lod_graph_rendering::*;
use rendiation_scene_rendering_gpu_gles::*;
use rendiation_shader_api::*;
use rendiation_texture_gpu_base::SamplerConvertExt;
use rendiation_webgpu_hook_utils::*;
use serde::{Deserialize, Serialize};
use tracing::*;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;
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
use egui_cx::use_egui_cx;
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

pub fn run_viewer_app(content_logic: impl Fn(&mut ViewerCx) + 'static) {
  setup_global_database(Default::default());
  global_database().enable_label_for_all_entity();

  register_scene_core_data_model();
  register_light_shadow_config();
  register_gui3d_extension_data_model();
  register_area_lighting_data_model();
  register_sky_env_data_model();
  register_scene_mesh_lod_graph_data_model();

  let init_config = ViewerInitConfig::from_default_json_or_default();

  // we do config override instead of gpu init override to reflect change in the init config
  #[cfg(feature = "webgl")]
  let init_config = {
    let mut init_config = init_config;
    init_config.init_only.wgpu_backend_select_override = Some(Backends::GL);
    init_config
  };

  run_application(
    init_config.init_only.wgpu_backend_select_override,
    move |cx| {
      use_egui_cx(cx, |cx, egui_cx| {
        use_viewer(cx, egui_cx, &init_config, |cx| {
          content_logic(cx);
        });
      });
    },
  );
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

  #[cfg(target_family = "wasm")]
  {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Info).unwrap();
    log::info!("init wasm");
  }

  #[cfg(not(target_family = "wasm"))]
  {
    env_logger::builder()
      .filter_level(log::LevelFilter::Info)
      .init();
  }

  run_viewer_app(|cx| {
    use_viewer_egui(cx);

    use_enable_screenshot(cx);

    stage_of_update(cx, 2, |cx| {
      use_viewer_gizmo(cx);
    });

    stage_of_update(cx, 1, |cx| {
      use_enable_gltf_io(cx);
      use_enable_obj_io(cx);

      sync_camera_view(cx);
      use_animation_player(cx);

      #[cfg(not(target_family = "wasm"))]
      test_persist_scope(cx);

      use_smooth_camera_motion(cx, |cx| {
        use_fit_camera_view(cx);
        use_camera_control(cx);
      });

      use_pick_scene(cx);
      use_scene_camera_helper(cx);
      use_scene_spotlight_helper(cx);

      use_mesh_tools(cx);
    });
  });
}

#[allow(dead_code)]
/// demo of how persistent scope api works
fn test_persist_scope(cx: &mut ViewerCx) {
  cx.suppress_scene_writer();
  use_persistent_db_scope(cx, |cx, persist_api| {
    cx.re_enable_scene_writer();

    // demo of how hydration works
    cx.use_state_init(|_| {
      let label = "root_scene";
      if let Some(handle) = persist_api.get_hydration_label(label) {
        println!("retrieve root persistent scene");
        unsafe { EntityHandle::from_raw(handle) }
      } else {
        println!("create new root persistent scene");
        let node = global_entity_of::<SceneEntity>()
          .entity_writer()
          .new_entity();

        persist_api.setup_hydration_label(label, node.into_raw());
        node
      }
    });

    core::hint::black_box(());

    cx.suppress_scene_writer();
  });
  cx.re_enable_scene_writer();
}
