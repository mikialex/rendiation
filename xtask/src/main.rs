use std::{ffi::OsString, process::ExitCode};

use anyhow::*;
use pico_args::Arguments;
use xshell::{cmd, Shell};

fn main() -> anyhow::Result<ExitCode> {
  let mut args = std::env::args_os().skip(1).collect::<Vec<_>>();
  let passthrough_args = args
    .iter()
    .position(|arg| arg == "--")
    .map(|pos| args.drain(pos..).skip(1).collect());
  let mut args = Arguments::from_vec(args);

  if args.contains(["-h", "--help"]) {
    eprint!("ok, but we don't have help message now!");
    return Ok(ExitCode::FAILURE);
  }

  let subcommand = args
    .subcommand()
    .context("Expected subcommand to be UTF-8")?;

  let shell = Shell::new().context("Couldn't create xshell shell")?;
  shell.change_dir(String::from(env!("CARGO_MANIFEST_DIR")) + "/..");

  match subcommand.as_deref() {
    Some("build-wasm") => build_wasm(shell, args, passthrough_args)?,
    Some(subcommand) => {
      bad_arguments!("Unknown subcommand: {}", subcommand)
    }
    None => {
      bad_arguments!("Expected subcommand")
    }
  }

  Ok(ExitCode::SUCCESS)
}

/// Helper macro for printing the help message, then bailing with an error message.
#[macro_export]
macro_rules! bad_arguments {
    ($($arg:tt)*) => {{
        anyhow::bail!($($arg)*)
    }};
}

fn build_wasm(
  shell: Shell,
  _args: Arguments,
  _passthrough_args: Option<Vec<OsString>>,
) -> anyhow::Result<()> {
  cmd!(
    shell,
    "cargo build --target wasm32-unknown-unknown -p viewer --release"
  )
  .quiet()
  .run()
  .context("Failed to build webgpu examples for wasm")?;

  cmd!(shell, "wasm-bindgen ./target/wasm32-unknown-unknown/release/viewer.wasm --target web --out-dir ./application/viewer-web/generated")
  .quiet()
  .run()
  .context("Failed to run wasm-bindgen for wasm")?;

  Ok(())
}
