use std::backtrace::Backtrace;
use std::ffi::c_char;
use std::io::Write;
use std::panic::PanicHookInfo;

use crate::*;

/// This must be called before any other rendiation c api
///
/// if trace_write_path is null_ptr, then the api tracing will be disabled
#[no_mangle]
pub extern "C" fn rendiation_init(trace_write_path: *const c_char) {
  std::panic::set_hook(Box::new(on_panic));

  env_logger::builder()
    .filter_level(log::LevelFilter::Info)
    .filter_module("wgpu_hal::dx12::device", log::LevelFilter::Warn)
    .init();

  setup_global_database(Default::default());
  global_database().enable_label_for_all_entity();

  register_viewer_content_data_model();

  setup_tracing(trace_write_path);
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
