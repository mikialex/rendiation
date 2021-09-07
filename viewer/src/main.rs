#![feature(capture_disjoint_fields)]
#![feature(array_methods)]
#![feature(min_specialization)]
#![feature(stmt_expr_attributes)]
#![allow(incomplete_features)]
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unreachable_code)]

mod scene;
use interphaser::Application;
pub use scene::*;

pub mod viewer;
pub use viewer::*;

fn main() {
  env_logger::builder().init();

  // let result = nfd::open_file_dialog(None, None).unwrap_or_else(|e| {
  //   panic!(e);
  // });

  // match result {
  //   nfd::Response::Okay(file_path) => println!("File path = {:?}", file_path),
  //   nfd::Response::OkayMultiple(files) => println!("Files {:?}", files),
  //   nfd::Response::Cancel => println!("User canceled"),
  // }

  let viewer = Viewer::new();
  let ui = create_ui();

  let viewer = futures::executor::block_on(Application::new(viewer, ui));
  viewer.run();
}
