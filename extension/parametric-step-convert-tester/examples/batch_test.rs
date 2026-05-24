//! Batch STEP test runner.
//!
//! Reads all `.stp` files from `step-generated-sample/`, converts each to
//! a `.glb` and SVG trim visualisation, and writes results to `test-output/`.
//!
//! ```sh
//! cargo run -p parametric-step-convert-tester --example batch_test
//! ```

use std::fs;
use std::path::PathBuf;

use parametric_step_convert_tester::{read_step, write_glb, write_surface_svg, GltfDoc};
use rendiation_parametric_rendering::mesh::{tessellate_curve, triangulate_trimmed_surface, TriangulationConfig};
use rendiation_parametric_rendering::validate_trim_boundary;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
  let sample_dir = crate_dir.join("step-generated-sample");
  let out_dir = crate_dir.join("test-output");
  if out_dir.exists() {
    fs::remove_dir_all(&out_dir)?;
  }
  let gltf_dir = out_dir.join("gltf");
  let svg_dir = out_dir.join("svg");
  fs::create_dir_all(&gltf_dir)?;
  fs::create_dir_all(&svg_dir)?;

  let mut step_files: Vec<PathBuf> = Vec::new();
  for entry in fs::read_dir(&sample_dir)? {
    let entry = entry?;
    let p = entry.path();
    if p.extension().map_or(false, |e| e == "stp") {
      step_files.push(p);
    }
  }
  step_files.sort();
  println!("found {} STEP files in {}", step_files.len(), sample_dir.display());

  let mut total_issues = 0usize;
  let mut total_surfaces = 0usize;

  for step_path in &step_files {
    let name = step_path.file_stem().unwrap().to_string_lossy();
    println!("\n=== {} ===", name);

    let result = read_step(step_path);
    result.print_errors();
    let data = &result.data;

    let n_surfaces = data.surfaces.len();
    total_surfaces += n_surfaces;
    println!(
      "  {} surfaces, {} 3D curves",
      n_surfaces,
      data.curves_3d.len()
    );

    // --- glTF ---
    let glb_path = gltf_dir.join(format!("{name}.glb"));
    let mut tri_config = TriangulationConfig::default();
    tri_config.ignore_surface_trim = false;
    let use_line_list = true;

    let mut doc = GltfDoc::new();

    for (surf_idx, trimmed) in data.surfaces.iter().enumerate() {
      let mesh = triangulate_trimmed_surface(trimmed, &tri_config);
      if mesh.indices.is_empty() {
        continue;
      }
      doc.create_surface_mesh(&mesh, &trimmed.debug_label, surf_idx);
    }

    for (inst_idx, &(surf_idx, matrix)) in data.surfaces_instance.iter().enumerate() {
      let trimmed = &data.surfaces[surf_idx];
      let label = format!("{}_inst{}", trimmed.debug_label, inst_idx);
      doc.add_surface_instance(surf_idx, matrix, &label);
    }

    for (curve_idx, curve) in data.curves_3d.iter().enumerate() {
      let pts = tessellate_curve(curve, 1e-3);
      if pts.len() >= 2 {
        doc.create_curve_mesh(&pts, curve_idx, use_line_list);
      }
    }

    for (inst_idx, &(curve_idx, matrix)) in data.curves_3d_instance.iter().enumerate() {
      let label = format!("curve_{}_inst{}", curve_idx, inst_idx);
      doc.add_curve_instance(curve_idx, matrix, &label);
    }

    let (root, bin) = doc.into_root();
    write_glb(&glb_path, &root, &bin)?;
    println!("  wrote {}", glb_path.display());

    // --- SVG ---
    let step_svg_dir = svg_dir.join(name.as_ref());
    fs::create_dir_all(&step_svg_dir)?;
    for s in &data.surfaces {
      if s.is_trimmed() {
        let issues = validate_trim_boundary(&s.debug_label, &s.trim_loops);
        if !issues.is_empty() {
          total_issues += issues.len();
        }
        write_surface_svg(&step_svg_dir, &s.debug_label, &s.trim_loops)?;
      }
    }
    println!("  wrote {} SVGs to {}", data.surfaces.iter().filter(|s| s.is_trimmed()).count(), step_svg_dir.display());
  }

  println!("\n=== DONE ===");
  println!("  {} files processed, {} total surfaces", step_files.len(), total_surfaces);
  if total_issues > 0 {
    println!("  {} validation issue(s) — see SVG output for details", total_issues);
  } else {
    println!("  0 validation issues");
  }
  Ok(())
}
