//! STEP → SVG trim-boundary viewer.
//!
//! Reads a STEP file, then writes one SVG per trimmed surface into an output
//! directory. Each SVG shows the [0,1]² parametric domain with every trim
//! boundary curve drawn in a distinct colour.
//!
//! ```sh
//! cargo run -p rendiation-parametric-rendering --example trim_svg -- input.stp output_dir/
//! ```

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;

use rendiation_parametric_rendering::step::{
  read_parametric_rendering_data_from_step, StepReadConfig,
};

/// Red→violet gradient following curve order.
/// Hue sweeps from 0° (red) to 270° (violet), saturation 85%, brightness 80%.
fn curve_color(idx: usize, total: usize) -> (u8, u8, u8) {
  let hue = if total <= 1 {
    0.0
  } else {
    (idx as f32 / (total - 1) as f32) * 270.0
  };
  let h = hue / 60.0;
  let c = 0.80 * 0.85; // value × sat
  let x = c * (1.0 - (h % 2.0 - 1.0).abs());
  let m = 0.80 - c;
  let (r, g, b) = match h as u32 % 6 {
    0 => (c, x, 0.0),
    1 => (x, c, 0.0),
    2 => (0.0, c, x),
    3 => (0.0, x, c),
    4 => (x, 0.0, c),
    _ => (c, 0.0, x),
  };
  (
    ((r + m) * 255.0) as u8,
    ((g + m) * 255.0) as u8,
    ((b + m) * 255.0) as u8,
  )
}

fn write_surface_svg(
  dir: &Path,
  label: &str,
  trim_loops: &[Vec<rendiation_parametric_rendering::QuadraticBezierCurve2d<f32>>],
) -> Result<(), Box<dyn std::error::Error>> {
  // Sanitise filename: replace characters that are awkward in paths.
  let fname = label
    .replace('/', "-")
    .replace('\\', "-")
    .replace(':', "-")
    .replace('*', "-")
    .replace('?', "-")
    .replace('"', "-")
    .replace('<', "-")
    .replace('>', "-")
    .replace('|', "-");
  let path = dir.join(format!("{fname}.svg"));

  // Map [0,1]² → pixel space with a margin.
  let margin = 20.0;
  let canvas = 540.0; // 500 px drawing + 20 px margin on each side
  let scale = canvas - 2.0 * margin;
  let to_svg = |x: f32, y: f32| -> (f64, f64) {
    (
      (margin + x as f64 * scale as f64),
      (margin + (1.0 - y as f64) * scale as f64), // flip V → SVG Y
    )
  };

  let mut svg = String::new();
  svg.push_str(&format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {canvas} {canvas}" width="{canvas}" height="{canvas}">"#
    ));
  svg.push('\n');

  // White background
  svg.push_str(&format!(
    r#"<rect x="0" y="0" width="{canvas}" height="{canvas}" fill="white"/>"#
  ));
  svg.push('\n');

  // [0,1]² domain — subtle filled quad (95% lightness)
  let (x0, y0) = to_svg(0.0, 0.0);
  let (x1, y1) = to_svg(1.0, 1.0);
  svg.push_str(&format!(
    r#"<rect x="{x0:.1}" y="{y1:.1}" width="{w:.1}" height="{h:.1}" fill="rgb(242,242,242)"/>"#,
    w = x1 - x0,
    h = y0 - y1,
  ));
  svg.push('\n');

  // Trim curves — flattened across all loops
  let total = trim_loops.iter().map(|l| l.len()).sum::<usize>();
  let mut ci = 0usize;
  for loop_curves in trim_loops {
    for curve in loop_curves {
      let (sx, sy) = to_svg(curve.start.x, curve.start.y);
      let (cx, cy) = to_svg(curve.ctrl.x, curve.ctrl.y);
      let (ex, ey) = to_svg(curve.end.x, curve.end.y);
      let (r, g, b) = curve_color(ci, total);
      ci += 1;
      svg.push_str(&format!(
              r#"<path d="M {sx:.4} {sy:.4} Q {cx:.4} {cy:.4} {ex:.4} {ey:.4}" stroke="rgb({r},{g},{b})" stroke-width="1.2" fill="none"/>"#
          ));
      svg.push('\n');
    }
  }

  svg.push_str("</svg>\n");

  fs::write(&path, svg)?;
  println!("  wrote {}", path.display());
  Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args: Vec<String> = env::args().collect();
  if args.len() < 3 {
    println!("usage: {} <input.stp> <output_dir/>", args[0]);
    std::process::exit(1);
  }

  let step_path = Path::new(&args[1]);
  let out_dir = Path::new(&args[2]);
  fs::create_dir_all(out_dir)?;

  println!("reading STEP: {}", step_path.display());
  let raw = fs::read_to_string(step_path)?;
  let step_str = rendiation_step_reader::step_utils::normalize_step(&raw);

  let config = StepReadConfig::default();
  let data = read_parametric_rendering_data_from_step(&step_str, config)?;

  println!("  {} trimmed surfaces", data.surfaces.len());

  // Uniqueness check
  let mut seen = HashSet::new();
  for s in &data.surfaces {
    if !seen.insert(&s.debug_label) {
      eprintln!("ERROR: duplicate debug_label: \"{}\"", s.debug_label);
      std::process::exit(1);
    }
  }

  // Validate trim boundaries
  let mut total_issues = 0usize;
  let mut surfaces_with_issues = 0usize;
  for s in &data.surfaces {
    if !s.is_trimmed() {
      continue;
    }
    let issues =
      rendiation_parametric_rendering::validate_trim_boundary(&s.debug_label, &s.trim_loops);
    if !issues.is_empty() {
      total_issues += issues.len();
      surfaces_with_issues += 1;
    }
  }
  if total_issues > 0 {
    eprintln!(
      "\n=== VALIDATION SUMMARY: {} issue(s) across {} surface(s) ===\n",
      total_issues, surfaces_with_issues
    );
  }

  for (i, s) in data.surfaces.iter().enumerate() {
    if !s.is_trimmed() {
      println!(
        "  [{}/{}] {} — skipping (untrimmed)",
        i + 1,
        data.surfaces.len(),
        s.debug_label
      );
      continue;
    }
    let n_curves: usize = s.trim_loops.iter().map(|l| l.len()).sum();
    println!(
      "  [{}/{}] {} — {} trim curves ({} loops)",
      i + 1,
      data.surfaces.len(),
      s.debug_label,
      n_curves,
      s.trim_loops.len()
    );
    write_surface_svg(out_dir, &s.debug_label, &s.trim_loops)?;
  }

  println!("done.");
  Ok(())
}
