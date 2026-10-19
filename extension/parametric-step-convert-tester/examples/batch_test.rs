//! Convert STEP files, print per file conversion and validation summaries, optionally write glTF
//! and per face SVG trim plots.
//!
//! ```sh
//! cargo run --release -p parametric-step-convert-tester --example batch_test -- [--gltf] [--svg] [--out DIR] [PATHS...]
//! ```
//!
//! Paths may be files or directories (searched recursively for .stp / .step). Without paths the
//! repo test sets are used: `temp/step-test` and `step-generated-sample`.

use std::path::{Path, PathBuf};

use parametric_step_convert_tester::mesh::TriangulationConfig;
use parametric_step_convert_tester::*;
use rendiation_step_to_bezier::ConvertConfig;
use rendiation_step_to_bezier::validate::validate;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
  let mut gltf = false;
  let mut svg = false;
  let mut out_dir = crate_dir.join("test-output");
  let mut inputs = Vec::new();
  let mut args = std::env::args().skip(1);
  while let Some(arg) = args.next() {
    match arg.as_str() {
      "--gltf" => gltf = true,
      "--svg" => svg = true,
      "--out" => out_dir = PathBuf::from(args.next().ok_or("--out needs a directory")?),
      _ => inputs.push(PathBuf::from(arg)),
    }
  }
  if inputs.is_empty() {
    inputs.push(crate_dir.join("../../temp/step-test"));
    inputs.push(crate_dir.join("step-generated-sample"));
  }

  let mut files = Vec::new();
  for input in &inputs {
    collect_step_files(input, &mut files);
  }
  files.sort();

  let config = ConvertConfig::default();
  let mut total_faces = 0;
  let mut total_failed = 0;
  for file in &files {
    let name = file
      .file_stem()
      .map_or_else(String::new, |s| s.to_string_lossy().to_string());
    let (session, elapsed) = convert_file(file, &config)?;
    let validation = validate(&session);
    print_summary(&name, &session, &validation, elapsed);
    total_faces += session.output.report.faces.len();
    total_failed += session
      .output
      .report
      .faces
      .iter()
      .filter(|f| f.result.is_err())
      .count();

    if gltf {
      std::fs::create_dir_all(&out_dir)?;
      let tri = TriangulationConfig {
        chord_tolerance: (session.ctx.tol * 10.).max(1e-4),
        ..Default::default()
      };
      let stats = write_session_glb(&session, &out_dir.join(format!("{name}.glb")), &tri)?;
      if stats.split_constraints > 0 {
        println!(
          "  glTF: {} constraint edges inserted split",
          stats.split_constraints
        );
      }
    }
    if svg {
      let dir = out_dir.join("svg").join(&name);
      std::fs::create_dir_all(&dir)?;
      write_face_svgs(&session.output, &dir)?;
    }
  }
  println!(
    "\n{} files, {} faces, {} failed",
    files.len(),
    total_faces,
    total_failed
  );
  Ok(())
}

fn collect_step_files(path: &Path, out: &mut Vec<PathBuf>) {
  if path.is_dir() {
    if let Ok(entries) = std::fs::read_dir(path) {
      for entry in entries.flatten() {
        collect_step_files(&entry.path(), out);
      }
    }
  } else if path
    .extension()
    .and_then(|e| e.to_str())
    .is_some_and(|e| e.eq_ignore_ascii_case("stp") || e.eq_ignore_ascii_case("step"))
  {
    out.push(path.to_path_buf());
  }
}
