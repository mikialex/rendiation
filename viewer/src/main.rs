#![feature(capture_disjoint_fields)]
#![feature(array_methods)]
#![feature(min_specialization)]
#![feature(stmt_expr_attributes)]
#![feature(type_alias_impl_trait)]
#![feature(option_result_unwrap_unchecked)]
#![feature(hash_raw_entry)]
#![allow(incomplete_features)]
#![allow(clippy::collapsible_match)]

pub mod scene;
pub use scene::*;

pub mod viewer;
pub use viewer::*;

use interphaser::Application;

fn main() {
  env_logger::builder().init();

  let viewer = Viewer::default();
  let ui = create_ui();

  let viewer = futures::executor::block_on(Application::new(viewer, ui));
  viewer.run();
}
