//! STEP → SVG trim-boundary viewer.
//!
//! Reads a STEP file, then writes one SVG per trimmed surface into an output
//! directory. Each SVG shows the [0,1]² parametric domain with every trim
//! boundary curve drawn in a distinct colour.
//!
//! ```sh
//! cargo run -p parametric-step-convert-tester --example trim_svg -- input.stp output_dir/
//! ```

use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::Path;

use parametric_step_convert_tester::{read_step, write_surface_svg};
use rendiation_parametric_rendering::validate_trim_boundary;

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
  let result = read_step(step_path);
  result.print_errors();
  let data = &result.data;

  println!("  {} trimmed surfaces", data.surfaces.len());

  let mut seen = HashSet::new();
  for s in &data.surfaces {
    if !seen.insert(&s.debug_label) {
      eprintln!("ERROR: duplicate debug_label: \"{}\"", s.debug_label);
      std::process::exit(1);
    }
  }

  let mut total_issues = 0usize;
  let mut surfaces_with_issues = 0usize;
  for s in &data.surfaces {
    if !s.is_trimmed() {
      continue;
    }
    let issues = validate_trim_boundary(&s.debug_label, &s.trim_loops);
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
      println!("  [{}/{}] {} — skipping (untrimmed)", i + 1, data.surfaces.len(), s.debug_label);
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
