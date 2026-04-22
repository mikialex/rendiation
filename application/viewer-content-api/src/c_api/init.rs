use std::backtrace::Backtrace;
use std::io::Write;
use std::panic::PanicHookInfo;

use crate::*;

/// call this to setup panic message writer when panic happens
#[no_mangle]
pub extern "C" fn rendiation_init() {
  std::panic::set_hook(Box::new(on_panic));

  env_logger::builder()
    .filter_level(log::LevelFilter::Info)
    .init();

  setup_global_database(Default::default());
  global_database().enable_label_for_all_entity();

  register_viewer_content_data_model();
}

fn on_panic(panic: &PanicHookInfo) {
  let backtrace = Backtrace::force_capture();
  let content = format!("{panic}\n{backtrace}\n");

  println!("rendiation panic");
  println!("{:?}", panic.payload_as_str());
  println!("{}", backtrace);

  let mut file = std::fs::OpenOptions::new()
    .create(true)
    .append(true)
    .open("rendiation_panic.txt");
  if let Ok(file) = &mut file {
    let content = format!("{content}\n\n-------------\n");
    let _result = file.write_all(content.as_bytes());
  }

  std::process::abort();
}
