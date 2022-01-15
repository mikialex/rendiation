#![feature(capture_disjoint_fields)]
#![feature(array_methods)]
#![feature(min_specialization)]
#![feature(stmt_expr_attributes)]
#![feature(type_alias_impl_trait)]
#![feature(hash_raw_entry)]
#![allow(incomplete_features)]
#![allow(clippy::collapsible_match)]
#![allow(clippy::field_reassign_with_default)]

use rendiation_scene::core::*;
use rendiation_scene::webgpu::*;

pub mod viewer;
pub use viewer::*;

pub mod app;
pub use app::*;

use interphaser::Application;

fn main() {
  #[cfg(target_arch = "wasm32")]
  {
    console_error_panic_hook::set_once();
    // console_log::init_with_level(Level::Debug).ok();

    let viewer = ViewerApplication::default();
    let ui = create_app();

    wasm_bindgen_futures::spawn_local(async move {
      let viewer = Application::new(viewer, ui).await;
      viewer.run();
    });
  }

  #[cfg(not(target_arch = "wasm32"))]
  {
    env_logger::builder().init();

    let viewer = ViewerApplication::default();
    let ui = create_app();

    let viewer = futures::executor::block_on(Application::new(viewer, ui));
    viewer.run();
  }
}
