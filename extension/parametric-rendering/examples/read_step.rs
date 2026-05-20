use std::env;
use std::fs;
use std::path::Path;
use std::time::Instant;

use rendiation_parametric_rendering::step::{
  read_parametric_rendering_data_from_step, StepReadConfig,
};
use rendiation_step_reader::step_utils::{normalize_step, visit_stp_files};

/// Parse STEP files and convert to parametric rendering data.
///
/// # Usage
///
/// ```sh
/// # Single file
/// cargo run -p rendiation-parametric-rendering --example read_step -- path/to/model.stp
///
/// # Directory: recursively scan all .stp/.step files
/// cargo run -p rendiation-parametric-rendering --example read_step -- path/to/dir/
/// ```
///
/// Prints per-file statistics: number of trimmed surfaces, 3D curves, parse/convert time.
fn main() {
  let args: Vec<String> = env::args().collect();
  if args.len() < 2 {
    println!("usage: {} <file.stp | directory>", args[0]);
    std::process::exit(1);
  }

  let path = Path::new(&args[1]);
  if !path.exists() {
    println!("path does not exist: {}", path.display());
    std::process::exit(1);
  }

  let config = if args.len() >= 3 && args[2] == "--loose" {
    StepReadConfig {
      tessellate_tolerance: 1.0,
      project_grid: 2,
      project_tolerance: 1.0,
      project_max_iter: 5,
      fit_tolerance: 1.0,
    }
  } else {
    StepReadConfig::default()
  };

  if path.is_dir() {
    let mut total = 0usize;
    let mut ok = 0usize;
    visit_stp_files(path, &mut |file_path| {
      total += 1;
      match process_file(file_path, &config) {
        ProcessResult::Success {
          surface_count,
          curve_count,
          duration,
        } => {
          ok += 1;
          println!(
            "OK  {}  ({} surfaces, {} curves, {:.0}ms)",
            file_path.display(),
            surface_count,
            curve_count,
            duration.as_secs_f64() * 1000.0
          );
        }
        ProcessResult::Error(e) => {
          println!("FAIL {}  ({})", file_path.display(), e);
        }
      }
    });
    println!("───");
    println!(
      "scanned {total} file(s), {ok} passed, {} failed",
      total - ok
    );
  } else {
    match process_file(path, &config) {
      ProcessResult::Success {
        surface_count,
        curve_count,
        duration,
      } => {
        println!("{}", path.display());
        println!("  trimmed surfaces: {surface_count}");
        println!("  3D curves:       {curve_count}");
        println!(
          "  time:            {:.0}ms",
          duration.as_secs_f64() * 1000.0
        );
      }
      ProcessResult::Error(e) => {
        println!("error: {e}");
        std::process::exit(1);
      }
    }
  }
}

enum ProcessResult {
  Success {
    surface_count: usize,
    curve_count: usize,
    duration: std::time::Duration,
  },
  Error(String),
}

fn process_file(path: &Path, config: &StepReadConfig) -> ProcessResult {
  let step_str = match fs::read_to_string(path) {
    Ok(s) => normalize_step(&s),
    Err(e) => return ProcessResult::Error(format!("cannot read file: {e}")),
  };

  let start = Instant::now();

  match read_parametric_rendering_data_from_step(&step_str, config.clone()) {
    Ok(data) => {
      let duration = start.elapsed();
      ProcessResult::Success {
        surface_count: data.surfaces.len(),
        curve_count: data.curves_3d.len(),
        duration,
      }
    }
    Err(e) => ProcessResult::Error(format!("{e}")),
  }
}
