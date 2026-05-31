//! STEP → glTF converter.
//!
//! Reads a STEP file, triangulates every trimmed surface and tessellates
//! 3D edge curves, then exports a single `.glb` file.
//!
//! ```sh
//! cargo run -p parametric-step-convert-tester --example gltf -- input.stp output.glb
//! ```

use std::env;
use std::path::Path;

use parametric_step_convert_tester::{read_step, write_glb, GltfDoc};
use rendiation_parametric_rendering::mesh::{
  tessellate_curve, triangulate_trimmed_surface, TriangulationConfig,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let args: Vec<String> = env::args().collect();
  if args.len() < 3 {
    println!("usage: {} <input.stp> <output.glb>", args[0]);
    std::process::exit(1);
  }

  // let step_path =
  //   Path::new("/Users/mikialex/dev/rendiation/extension/parametric-step-convert-tester/step-generated-sample/cylinder.stp");
  // let gltf_path = Path::new("/Users/mikialex/dev/rendiation/out.glb");

  let step_path = Path::new(&args[1]);
  let gltf_path = Path::new(&args[2]);
  let use_line_list = true;

  println!("reading STEP: {}", step_path.display());
  let result = read_step(step_path);
  result.print_errors();
  let data = &result.data;

  println!(
    "  {} unique surfaces ({} instances), {} unique 3D curves ({} instances)",
    data.surfaces.len(),
    data.surfaces_instance.len(),
    data.curves_3d.len(),
    data.curves_3d_instance.len()
  );

  let mut tri_config = TriangulationConfig::default();
  tri_config.ignore_surface_trim = false;

  let mut doc = GltfDoc::new();

  for (surf_idx, trimmed) in data.surfaces.iter().enumerate() {
    println!(
      "  triangulating surface {}/{} [{}]{}",
      surf_idx + 1,
      data.surfaces.len(),
      trimmed.debug_label,
      if !trimmed.is_trimmed() {
        " (untrimmed)"
      } else {
        ""
      }
    );
    let mesh = triangulate_trimmed_surface(trimmed, &tri_config);
    if mesh.indices.is_empty() {
      println!("    skipped (empty triangulation)");
      continue;
    }
    println!(
      "    {} vertices, {} triangles",
      mesh.positions.len(),
      mesh.indices.len()
    );
    doc.create_surface_mesh(&mesh, &trimmed.debug_label, surf_idx);
  }

  for (inst_idx, &(surf_idx, matrix)) in data.surfaces_instance.iter().enumerate() {
    let trimmed = &data.surfaces[surf_idx];
    let label = format!("{}_inst{}", trimmed.debug_label, inst_idx);
    doc.add_surface_instance(surf_idx, matrix, &label);
  }

  if !data.curves_3d.is_empty() {
    println!(
      "  tessellating {} unique 3D curves...",
      data.curves_3d.len()
    );
  }
  for (curve_idx, curve) in data.curves_3d.iter().enumerate() {
    let pts = tessellate_curve(curve, 1e-3);
    if pts.len() < 2 {
      println!(
        "    curve {}/{}: skipped (too few points)",
        curve_idx + 1,
        data.curves_3d.len()
      );
      continue;
    }
    println!(
      "    curve {}/{}: {} line points",
      curve_idx + 1,
      data.curves_3d.len(),
      pts.len()
    );
    doc.create_curve_mesh(&pts, curve_idx, use_line_list);
  }

  for (inst_idx, &(curve_idx, matrix)) in data.curves_3d_instance.iter().enumerate() {
    let label = format!("curve_{}_inst{}", curve_idx, inst_idx);
    doc.add_curve_instance(curve_idx, matrix, &label);
  }

  let (root, bin) = doc.into_root();
  println!(
    "writing glTF: {} ({} meshes, {} nodes, {}K binary)",
    gltf_path.display(),
    root.meshes.len(),
    root.nodes.len(),
    bin.len() / 1024
  );
  write_glb(gltf_path, &root, &bin)?;
  println!("done.");
  Ok(())
}
