#![feature(impl_trait_in_assoc_type)]
#![feature(specialization)]
#![feature(stmt_expr_attributes)]
#![feature(type_alias_impl_trait)]
#![feature(hash_raw_entry)]
#![allow(clippy::collapsible_match)]

use std::alloc::System;

use rendiation_scene_core::*;
use rendiation_scene_webgpu::*;

mod viewer;
pub use viewer::*;

mod app;
pub use app::*;
use heap_tools::*;
use interphaser::{run_gui, WindowSelfState};

#[global_allocator]
static GLOBAL_ALLOCATOR: PreciseAllocationHook<System> = PreciseAllocationHook::new(System);

fn main() {
  setup_active_plane(Default::default());
  register_viewer_extra_scene_features();

  let window_init_config = WindowSelfState {
    size: (1200., 800.).into(),
    title: "viewer".to_owned(),
    position: (50., 50.).into(),
  };

  let ui = create_app();
  let running_gui = run_gui(ui, window_init_config);

  #[cfg(not(target_arch = "wasm32"))]
  {
    env_logger::builder().init();

    futures::executor::block_on(running_gui)
  }

  #[cfg(target_arch = "wasm32")]
  {
    console_error_panic_hook::set_once();
    console_log::init_with_level(Level::Debug).ok();

    wasm_bindgen_futures::spawn_local(running_gui);
  }
}
