use std::backtrace::Backtrace;
use std::io::Write;
use std::panic::PanicHookInfo;

/// call this to setup panic message writer when panic happens
#[no_mangle]
pub extern "C" fn setup_panic_message_file_writer() {
  std::panic::set_hook(Box::new(on_panic));
}

fn on_panic(panic: &PanicHookInfo) {
  let backtrace = Backtrace::force_capture();
  let content = format!("{panic}\n{backtrace}\n");

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
