#![feature(capture_disjoint_fields)]
#![feature(array_methods)]
#![feature(min_specialization)]
#![allow(incomplete_features)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unreachable_code)]

mod scene;
pub use scene::*;

#[macro_use]
pub mod ui;
pub use ui::*;

pub mod viewer;
pub use viewer::*;

fn main() {
  env_logger::builder().init();

  let viewer = Viewer::new();
  let ui = create_ui();

  let viewer = futures::executor::block_on(Application::new(viewer, ui));
  viewer.run();
}
