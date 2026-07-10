#![feature(impl_trait_in_assoc_type)]
#![feature(file_buffered)]
#![feature(stmt_expr_attributes)]
#![feature(type_alias_impl_trait)]
#![feature(ptr_metadata)]
#![feature(iter_array_chunks)]
#![allow(clippy::collapsible_match)]
#![feature(cold_path)]

use std::any::Any;
use std::future::Future;
use std::hash::Hash;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Poll;
use std::task::Waker;

use bytemuck::*;
use database::*;
use database_tracing::TraceWriter;
use event_source::*;
use fast_hash_collection::*;
use futures::FutureExt;
use futures::StreamExt;
use rendiation_algebra::*;
use rendiation_geometry::*;
use rendiation_gui_3d::*;
use rendiation_viewer_content::*;
use tracing::*;
use winit::{
  event::{Event, WindowEvent},
  event_loop::EventLoop,
  window::Window,
};

mod app_loop;
mod egui_cx;
mod viewer;

use app_loop::*;
use egui_cx::use_egui_cx;
use heap_tools::*;
use rendiation_texture_core::*;
use rendiation_webgpu::*;
pub use viewer::*;

#[cfg(feature = "mimalloc")]
#[allow(unused)]
type BaseAllocator = mimalloc::MiMalloc;
#[cfg(feature = "mimalloc")]
#[allow(unused)]
const BASE_ALLOCATOR: BaseAllocator = mimalloc::MiMalloc;

#[cfg(not(feature = "mimalloc"))]
#[allow(unused)]
type BaseAllocator = std::alloc::System;
#[cfg(not(feature = "mimalloc"))]
#[allow(unused)]
const BASE_ALLOCATOR: BaseAllocator = std::alloc::System;

// global_allocator priority: dhat-heap-profiling > tracy-heap-debug > base
#[cfg(feature = "dhat-heap-profiling")]
#[global_allocator]
static GLOBAL_ALLOCATOR: PreciseAllocationStatistics<dhat::Alloc> =
  PreciseAllocationStatistics::new(dhat::Alloc);

#[cfg(all(not(feature = "dhat-heap-profiling"), feature = "tracy-heap-debug"))]
#[global_allocator]
static GLOBAL_ALLOCATOR: PreciseAllocationStatistics<
  tracing_tracy::client::ProfiledAllocator<BaseAllocator>,
> = PreciseAllocationStatistics::new(tracing_tracy::client::ProfiledAllocator::new(
  BASE_ALLOCATOR,
  64,
));

#[derive(Debug)]
pub enum ViewerTracingEvent {
  Render,
}

impl database_tracing::TraceReplayTarget for ViewerTracingEvent {
  fn is_replay_target(&self) -> bool {
    match self {
      ViewerTracingEvent::Render => true,
    }
  }
}

impl database_tracing::TraceIO for ViewerTracingEvent {
  fn write_len(&self) -> usize {
    1
  }

  fn write(&self, w: &mut impl std::io::prelude::Write) -> std::io::Result<usize> {
    match self {
      ViewerTracingEvent::Render => {
        w.write_all(&[0u8])?;
        Ok(1)
      }
    }
  }

  fn read(source: &mut impl std::io::prelude::Read) -> std::io::Result<Self>
  where
    Self: Sized,
  {
    let mut tag = [0u8; 1];
    source.read_exact(&mut tag)?;
    match tag[0] {
      0 => Ok(ViewerTracingEvent::Render),
      other => Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        format!("unknown ViewerTracingEvent tag: {}", other),
      )),
    }
  }
}

#[cfg(all(
  not(feature = "dhat-heap-profiling"),
  not(feature = "tracy-heap-debug")
))]
#[global_allocator]
static GLOBAL_ALLOCATOR: PreciseAllocationStatistics<BaseAllocator> =
  PreciseAllocationStatistics::new(BASE_ALLOCATOR);

pub fn run_viewer_app(content_logic: impl Fn(&mut ViewerCx) + 'static) {
  setup_global_database(Default::default());
  global_database().enable_label_for_all_entity();

  register_viewer_content_data_model();

  let init_config = ViewerInitConfig::from_default_json_or_default();

  let trace_event_notifier = if let Some(ref trace_write_path) =
    init_config.init_only.enable_tracing_and_tracing_write_path
  {
    use database_tracing::*;
    let writer = FileTraceWriter::<TracingMessage<ViewerTracingEvent>>::new(trace_write_path);
    let trace_event_notifier = start_tracing(&global_database(), writer);
    Some(trace_event_notifier)
  } else {
    None
  };

  // we do config override instead of gpu init override to reflect change in the init config
  #[cfg(target_family = "wasm")]
  let init_config = {
    let search = web_sys::window().unwrap().location().search();
    let params = web_sys::UrlSearchParams::new_with_str(&search.unwrap()).unwrap();

    let mut init_config = init_config;
    init_config.init_only.wgpu_backend_select_override =
      Some(Backends::GL | Backends::BROWSER_WEBGPU);

    if let Some(value) = params.get("host_driven_draw") {
      if value == "true" {
        init_config.init_only.enable_indirect_storage_combine = true;
        init_config
          .init_only
          .using_texture_as_storage_buffer_for_indirect_rendering = true;
        init_config.using_host_driven_indirect_draw = true;
        init_config.raster_backend_type = RasterizationRenderBackendType::Indirect
      }
    }

    if let Some(value) = params.get("force_webgl2") {
      if value == "true" {
        #[cfg(feature = "support-webgl")]
        {
          init_config.init_only.wgpu_backend_select_override = Some(Backends::GL);
          log::warn!("force using webgl2 by url param");
        }
        #[cfg(not(feature = "support-webgl"))]
        {
          panic!("force_webgl2 is not supported in current build");
        }
      }
    }
    init_config
  };

  let gpu_config = init_config.make_gpu_platform_config();

  run_application(gpu_config, move |cx| {
    use_egui_cx(cx, |cx, egui_cx| {
      use_viewer(
        cx,
        egui_cx,
        &init_config,
        &|message| {
          if let Some(notifier) = &trace_event_notifier {
            notifier.write_message(database_tracing::TracingMessage::Event(message));
          }
        },
        |cx| {
          content_logic(cx);
        },
      );
    });
  });
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
      .filter_module("wgpu_hal::dx12::device", log::LevelFilter::Warn)
      .init();
  }

  run_viewer_app(|cx| {
    setup_new_frame_allocator(10 * 1024 * 1024);

    use_viewer_egui(cx);

    use_enable_screenshot(cx);

    stage_of_update(cx, 2, |cx| {
      // todo, support group
      let select = cx.viewer.selection.selected_model.if_single();
      widget_root(cx, |cx| {
        use_viewer_gizmo(cx, select);
      });
    });

    stage_of_update(cx, 1, |cx| {
      // test_db_rc(cx);

      use_enable_gltf_io(cx);
      use_enable_obj_io(cx);
      use_enable_trace_io(cx);
      use_test_content_panel(cx);

      sync_camera_view(cx);

      use_viewer_examples(cx);

      // this must be called before per_camera_per_viewport
      use_egui_tile_for_viewer_viewports(cx);

      inject_picker(cx, |cx| {
        use_pick_scene(cx);
        use_scene_camera_helper(cx);
        use_scene_light_helper(cx);
      });

      per_camera_per_viewport_scope(cx, false, |cx, camera_with_viewports| {
        let cv = camera_with_viewports;
        use_smooth_camera_motion(cx, cv.camera_node, cv.camera, |cx| {
          use_fit_camera_view(cx, cv.camera, cv.camera_node);
          use_camera_control(cx, cv);
          use_camera_proj_switch(cx);
        });
      });

      use_animation_player(cx);

      // #[cfg(not(target_family = "wasm"))]
      // test_persist_scope(cx);

      use_mesh_tools(cx);
    });
  });
}

#[allow(dead_code)]
fn test_db_rc(cx: &mut ViewerCx) {
  let (cx, config) = cx.use_plain_state_init(|_| {
    let mut set = fast_hash_collection::FastHashSet::default();
    set.insert(SceneNodeParentIdx::component_id());
    set.insert(SceneModelBelongsToScene::component_id());
    set.insert(AttributesMeshEntityVertexBufferRelationRefAttributesMeshEntity::component_id());
    set
  });

  let change = use_db_all_entity_ref_count_change(cx, config).use_assure_result(cx);
  if let Some(_change) = change.if_resolve_stage() {
    // println!("ref count change: {:#?}", change.len());
  }
}

fn per_camera_per_viewport_scope(
  cx: &mut ViewerCx,
  consider_debug_view_camera_override: bool,
  logic: impl Fn(&mut ViewerCx, &CameraViewportAccess),
) {
  cx.next_key_scope_root();

  let surface_content = &cx.active_surface_content;

  for cv in per_camera_per_viewport(
    &surface_content.viewports,
    consider_debug_view_camera_override,
  ) {
    cx.keyed_scope(&cv.camera, |cx| {
      logic(cx, &cv);
    });
  }
}
