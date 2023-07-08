#![feature(impl_trait_in_assoc_type)]
#![feature(array_methods)]
#![feature(specialization)]
#![feature(stmt_expr_attributes)]
#![feature(type_alias_impl_trait)]
#![feature(hash_raw_entry)]
#![allow(incomplete_features)]
#![allow(clippy::collapsible_match)]
#![allow(clippy::field_reassign_with_default)]

use rendiation_scene_core::*;
use rendiation_scene_webgpu::*;

pub mod viewer;
pub use viewer::*;

pub mod app;
pub use app::*;
use interphaser::{run_gui, WindowSelfState};

fn main() {
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
