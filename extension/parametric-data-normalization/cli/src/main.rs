use std::path::{Path, PathBuf};
use std::process::ExitCode;

use parametric_normalization_cli::*;
use parametric_normalized_model::{PNM_EXTENSION, ParametricNormalizedModel};
use rendiation_algebra::*;
use step_to_bezier::ConvertConfig;
use step_to_bezier::validate::validate;

const USAGE: &str = "\
usage:
  parametric-normalization-cli convert INPUT -o OUTPUT [-o OUTPUT...] [--svg DIR] [--tolerance T]
      [--camera fit[:ZOOM]|EX,EY,EZ,TX,TY,TZ] [--viewport WxH] [--fov DEG] [--pixel-error PX]
      [--edge-share S] [--max-triangle PX] [--k-max K] [--untrimmed-only] [--verify]
      INPUT: .stp / .step / .pnm, OUTPUT: .pnm / .glb
      --camera makes .glb outputs the view dependent adaptive tessellation (trim ignored,
      trimmed patches washed out) seen from that camera, --verify checks its error and edges
  parametric-normalization-cli batch [--gltf] [--svg] [--pnm] [--out DIR] [PATHS...]
      convert every .stp / .step under PATHS (default: the repo test sets) and print summaries,
      --pnm also checks that each model survives a .pnm write and read unchanged";

type CliResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn main() -> ExitCode {
  let args: Vec<_> = std::env::args().skip(1).collect();
  let result = match args.first().map(String::as_str) {
    Some("convert") => convert(&args[1..]),
    Some("batch") => batch(&args[1..]),
    _ => {
      eprintln!("{USAGE}");
      return ExitCode::FAILURE;
    }
  };
  match result {
    Ok(()) => ExitCode::SUCCESS,
    Err(e) => {
      eprintln!("error: {e}");
      ExitCode::FAILURE
    }
  }
}

fn convert(args: &[String]) -> CliResult {
  let mut input = None;
  let mut outputs = Vec::new();
  let mut svg_dir = None;
  let mut config = ConvertConfig::default();
  let mut view_dependent = false;
  let mut view_options = ViewOptions::default();
  let mut args = args.iter();
  while let Some(arg) = args.next() {
    match arg.as_str() {
      "-o" | "--output" => outputs.push(PathBuf::from(next_value(&mut args, arg)?)),
      "--svg" => svg_dir = Some(PathBuf::from(next_value(&mut args, arg)?)),
      "--tolerance" => config.tolerance = Some(next_value(&mut args, arg)?.parse()?),
      "--camera" => {
        view_options.placement = parse_camera(next_value(&mut args, arg)?)?;
        view_dependent = true;
      }
      "--viewport" => {
        let value = next_value(&mut args, arg)?;
        let (w, h) = value
          .split_once('x')
          .ok_or(format!("bad viewport {value}"))?;
        view_options.viewport = Vec2::new(w.parse()?, h.parse()?);
      }
      "--fov" => view_options.fov_y_degrees = next_value(&mut args, arg)?.parse()?,
      "--pixel-error" => view_options.config.pixel_error = next_value(&mut args, arg)?.parse()?,
      "--edge-share" => {
        view_options.config.edge_error_share = next_value(&mut args, arg)?.parse()?
      }
      "--max-triangle" => {
        view_options.config.max_triangle_size = next_value(&mut args, arg)?.parse()?
      }
      "--k-max" => view_options.config.k_max = next_value(&mut args, arg)?.parse()?,
      "--untrimmed-only" => view_options.untrimmed_only = true,
      "--verify" => view_options.verify = true,
      _ if input.is_none() => input = Some(PathBuf::from(arg)),
      _ => return Err(format!("unexpected argument {arg}\n{USAGE}").into()),
    }
  }
  let input = input.ok_or(format!("missing input\n{USAGE}"))?;
  let view = view_dependent.then_some(view_options);
  if outputs.is_empty() && svg_dir.is_none() {
    return Err(format!("no output given\n{USAGE}").into());
  }

  let model = if has_extension(&input, &[PNM_EXTENSION]) {
    ParametricNormalizedModel::load_pnm(&input)?
  } else if has_extension(&input, &["stp", "step"]) {
    let (session, elapsed) = convert_file(&input, &config)?;
    print_summary(&file_name(&input), &session, &validate(&session), elapsed);
    session.model
  } else {
    return Err(format!("unsupported input {}", input.display()).into());
  };

  for output in &outputs {
    if has_extension(output, &[PNM_EXTENSION]) {
      model.save_pnm(output)?;
    } else if has_extension(output, &["glb"]) {
      match &view {
        Some(view) => write_view_dependent_glb(&model, output, view)?,
        None => write_glb_reporting(&model, output)?,
      }
    } else {
      return Err(format!("unsupported output {}", output.display()).into());
    }
    println!("wrote {}", output.display());
  }
  if let Some(dir) = svg_dir {
    std::fs::create_dir_all(&dir)?;
    let count = write_face_svgs(&model, &dir)?;
    println!("wrote {count} face svgs to {}", dir.display());
  }
  Ok(())
}

fn batch(args: &[String]) -> CliResult {
  let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
  let mut gltf = false;
  let mut svg = false;
  let mut pnm = false;
  let mut out_dir = crate_dir.join("test-output");
  let mut inputs = Vec::new();
  let mut args = args.iter();
  while let Some(arg) = args.next() {
    match arg.as_str() {
      "--gltf" => gltf = true,
      "--svg" => svg = true,
      "--pnm" => pnm = true,
      "--out" => out_dir = PathBuf::from(next_value(&mut args, arg)?),
      _ => inputs.push(PathBuf::from(arg)),
    }
  }
  if inputs.is_empty() {
    inputs.push(crate_dir.join("../../../temp/step-test"));
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
  let mut pnm_mismatches = 0;
  for file in &files {
    let name = file_name(file);
    let (session, elapsed) = convert_file(file, &config)?;
    print_summary(&name, &session, &validate(&session), elapsed);
    total_faces += session.report.faces.len();
    total_failed += session
      .report
      .faces
      .iter()
      .filter(|f| f.result.is_err())
      .count();

    if pnm {
      std::fs::create_dir_all(&out_dir)?;
      let path = out_dir.join(format!("{name}.{PNM_EXTENSION}"));
      session.model.save_pnm(&path)?;
      let size = std::fs::metadata(&path)?.len();
      if ParametricNormalizedModel::load_pnm(&path)? == session.model {
        println!("  pnm: {:.1} KiB, round trip ok", size as f64 / 1024.);
      } else {
        pnm_mismatches += 1;
        println!("  pnm: round trip MISMATCH");
      }
    }
    if gltf {
      std::fs::create_dir_all(&out_dir)?;
      write_glb_reporting(&session.model, &out_dir.join(format!("{name}.glb")))?;
    }
    if svg {
      let dir = out_dir.join("svg").join(&name);
      std::fs::create_dir_all(&dir)?;
      write_face_svgs(&session.model, &dir)?;
    }
  }
  println!(
    "\n{} files, {} faces, {} failed",
    files.len(),
    total_faces,
    total_failed
  );
  if pnm_mismatches > 0 {
    return Err(format!("{pnm_mismatches} pnm round trip mismatches").into());
  }
  Ok(())
}

fn write_glb_reporting(model: &ParametricNormalizedModel, path: &Path) -> CliResult {
  let stats = write_model_glb(model, path, &default_triangulation_config(model))?;
  if stats.split_constraints > 0 {
    println!(
      "  glTF: {} constraint edges inserted split",
      stats.split_constraints
    );
  }
  Ok(())
}

fn parse_camera(value: &str) -> CliResult<CameraPlacement> {
  if let Some(zoom) = value.strip_prefix("fit") {
    let zoom = zoom.strip_prefix(':').map_or(Ok(1.), str::parse)?;
    return Ok(CameraPlacement::Fit { zoom });
  }
  let v: Vec<f64> = value.split(',').map(str::parse).collect::<Result<_, _>>()?;
  if v.len() != 6 {
    return Err(format!("--camera needs fit[:ZOOM] or EX,EY,EZ,TX,TY,TZ, got {value}").into());
  }
  Ok(CameraPlacement::LookAt {
    eye: Vec3::new(v[0], v[1], v[2]),
    target: Vec3::new(v[3], v[4], v[5]),
  })
}

fn next_value<'a>(args: &mut impl Iterator<Item = &'a String>, flag: &str) -> CliResult<&'a str> {
  Ok(args.next().ok_or(format!("{flag} needs a value"))?)
}

fn has_extension(path: &Path, extensions: &[&str]) -> bool {
  path
    .extension()
    .and_then(|e| e.to_str())
    .is_some_and(|e| extensions.iter().any(|x| e.eq_ignore_ascii_case(x)))
}

fn file_name(path: &Path) -> String {
  path
    .file_stem()
    .map_or_else(String::new, |s| s.to_string_lossy().to_string())
}

fn collect_step_files(path: &Path, out: &mut Vec<PathBuf>) {
  if path.is_dir() {
    if let Ok(entries) = std::fs::read_dir(path) {
      for entry in entries.flatten() {
        collect_step_files(&entry.path(), out);
      }
    }
  } else if has_extension(path, &["stp", "step"]) {
    out.push(path.to_path_buf());
  }
}
